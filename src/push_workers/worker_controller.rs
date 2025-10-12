use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel, Receiver, Sender, TrySendError};

use crate::collector::Collector;
use crate::errors::WorkThreadError;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::{CMesg, Coordination, WorkerThread};

// With 2 queues, the thread always has a backup to work on and does not wait
const BUF_SIZE_CHANNEL:usize = 1;
const INITIAL_WORKERS:usize = 1;

pub struct WorkerController<F,V,T,I> 
where F: Fn(V) -> T + Send + Sync,
V: Send + Sync,
T: Send + Sync,    
I:AtomicIterator<AtomicItem = V> + Send + Sized 
{
    f:Arc<RwLock<F>>,
    values: I,
    all_sender:Sender<ThreadMesg>,
    all_receiver: Receiver<ThreadMesg>,
    buf_size: usize
}

type ThreadInfo<'scope, T,V> = WorkerThread<'scope,V,T>;

impl<F,V,T,I>  WorkerController<F,V,T,I>
where F: Fn(V) -> T + Send + Sync,
V: Send + Sync,
T: Send + Sync,    
I:AtomicIterator<AtomicItem = V> + Send + Sized  
{

    pub fn new(f:F, values:I) -> Self 
    {
        //send/receive channel to reply on success
        let (all_sender, all_receiver) = async_channel::<ThreadMesg>();
        
        Self {
            f: Arc::new(RwLock::new(f)),
            values,
            all_sender,
            all_receiver,
            buf_size: BUF_SIZE_CHANNEL
        }
    }

    fn next_task(&mut self) -> Option<CMesg<V>> {
        
        let vec = self.values.atomic_pull(); 
        vec.map(CMesg::run_task)                                          
    }

    fn add_thread<'scope,'env>(&mut self, scope:&'scope std::thread::Scope<'scope, 'env>, threads:&mut Vec<ThreadInfo<'scope,T,V>>) -> Result<(),WorkThreadError>
    where T: 'scope,
    V: 'scope,
    F: 'scope
    {
        let worker_sender_clone = self.all_sender.clone();                                                         
        let arc_f_clone: Arc<RwLock<F>> = self.f.clone();       
        match WorkerThread::launch(scope,worker_sender_clone,threads.len(),self.buf_size, arc_f_clone) {
            Ok(t) =>  {                                                    
                threads.push(t);  
                Ok(())                                                                                                        
            } 
            Err(e) =>  {
                Err(WorkThreadError::ThreadAdd(e.to_string()))
            }
        }                              
    }

    pub fn run<C>(&mut self) -> Result<C,WorkThreadError>
    where C: Collector<T>,    
    {                        
        let max_threads = crate::utils::max_threads();              

        std::thread::scope(            
            |s: &Scope<'_, '_>| {                   
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                                                                                                                                                     
                
                // Generate initial worker threads as you need at least 1 by default. Record control time
                let tm = std::time::Instant::now();   
                for _ in 0..INITIAL_WORKERS {
                    self.add_thread(s, &mut threads)?;                   
                }                                                                                                            
                let mut control_time = tm.elapsed().as_nanos();                          
                                
                let mut task:CMesg<V> = self.next_task().unwrap_or(CMesg::done());                                                            
                           
                let mut process_time = std::time::Instant::now(); 
                let mut elapsed_monitored_time = 0;

                let mut free_threads:VecDeque<usize> = VecDeque::new();

                loop {                                                                                                              
                    if let Ok(ThreadMesg::Free(pos, _)) = self.all_receiver.try_recv() {                                                 
                        elapsed_monitored_time = process_time.elapsed().as_nanos();                        
                        process_time = std::time::Instant::now();                      
                        
                        let thread= &mut threads[pos];                        
                        
                        task = if let Some((task,_)) = self.send_task(thread,  task) {                                
                            task
                        } else {
                            free_threads.push_back(pos);
                            break;
                        };                                                                                               
                    } 
 
                    if threads.len() < max_threads && elapsed_monitored_time as f64 > control_time as f64  { 
                        
                        let tm = std::time::Instant::now();
                        self.add_thread(s, &mut threads)?;                                                    
                        control_time = tm.elapsed().as_nanos();                                
                    }                                                                                                                                                                                                                                                           
                }
                                              
                if threads.len() > 2 {
                    let mut task:Option<CMesg<V>> = None;
                    let mut min_rate_change:f64;
                    let mut maxpos:isize = -1;
                    let mut jobstatus = (0..threads.len())
                    .map(|idx| (threads[idx].primary_q.len(),0.0))
                    .collect::<Vec<(usize,f64)>>();
                    loop {                                         
                        if task.is_none() {
                            min_rate_change = 1.0;
                            maxpos = -1;
                            let mut maxlen:usize = 0;
                            for (pos,thread) in &mut threads.iter_mut().enumerate() {
                                let currlen = thread.primary_q.len();
                                let (lastlen, _rate_of_change) = jobstatus[pos];                            
                                let curr_rate_change = if (lastlen == 0) || (lastlen < currlen) { 1.0 } else { (lastlen - currlen) as f64 / lastlen as f64 };                                                        
                                jobstatus[pos] = (currlen,curr_rate_change);

                                if curr_rate_change < min_rate_change  && currlen > 0 &&  currlen > maxlen {
                                    // print!(" ({}::{}) ",pos,curr_rate_change);
                                    maxpos = pos as isize;
                                    min_rate_change = curr_rate_change;
                                    maxlen = currlen;
                                }                           
                            }

                            if maxpos == -1 { 
                                // println!("No max available");
                                break; }

                            if let Some(pending) = threads[maxpos as usize].primary_q.steal_half() {
                                // println!("Out: {}->Len:{}",maxpos,pending.len());
                                task = Some(CMesg::run_task(pending));                                                     
                            } else {
                                // println!("Steal failed: ");
                            }
                        }                    

                        if task.is_some() {                        
                            if let Some(free_pos) = free_threads.pop_front() {
                                if free_pos != maxpos as usize {
                                    let new_task = task.unwrap();
                                    let free_thread = &mut threads[free_pos];                            
                                    if let Err(fail_task) = self.send_leaked_task(free_thread, new_task) {
                                        // println!("Failed: From:{} To:{}",maxpos,free_pos);
                                        task = Some(fail_task);
                                    } else {
                                        // println!("Success: From:{} To:{}",maxpos,free_pos);                                        
                                        task = None;
                                    }
                                } else {
                                    // println!("Retry:From:{} To:{}",maxpos,free_pos);
                                    free_threads.push_back(free_pos);
                                }
                                
                            }
                        } else {
                            break;
                        }                                                            

                        // Get the next free thread
                        while let Ok(msg) = self.all_receiver.try_recv() {
                            if let ThreadMesg::Free(pos, _) = msg {   
                                // println!("Freed:{}",pos); 
                                // if !ignore_threads[pos] {
                                    free_threads.push_back(pos);  
                                // }                                                                             
                            }
                        }                                                                       
                    }     
                }
                                                              
                // println!("work stealing = {}",tm.elapsed().as_micros());    

                //join all threads                     
                for thread in threads {                                                                    
                    if let Ok(res) = thread.join(){
                        results.extend(res.into_iter());
                    };                                                                                                     
                }  
                // println!("time to join = {}",tm.elapsed().as_micros());                    
                       
                Ok(results)
            }
            
        )        
    }

    fn send_leaked_task<'scope>(&mut self, thread:&mut WorkerThread<'scope, V,T>, task:CMesg<V>) -> Result<(),CMesg<V>>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    { 
        if let Err(TrySendError::Full(e)) = thread.try_send(task) {
            return Err(e);
        };

        Ok(())
    }
    

    fn send_task<'scope>(&mut self, thread:&mut WorkerThread<'scope, V,T>, task:CMesg<V>) -> Option<(CMesg<V>,bool)>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {                
        let task = match thread.try_send(task) {
            Ok(_) => { 
                if let Some(task) = self.next_task(){
                    (task,true)
                } else {
                    return None;
                }                                 
            }
            Err(e) => {                
                if let TrySendError::Full(mesg) = e {
                    (mesg,false)
                } else {
                    (CMesg {
                        msgtype:Coordination::Done,
                        msg: None
                    },false)
                }
            }
        };        
        
        Some(task)
    }
}

