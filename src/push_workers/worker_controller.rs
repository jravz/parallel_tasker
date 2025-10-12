use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel, Receiver, Sender, TrySendError};

use crate::collector::Collector;
use crate::errors::WorkThreadError;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::{CMesg, Coordination, MessageValue, WorkerThread};

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
            values: values,
            all_sender,
            all_receiver,
            buf_size: BUF_SIZE_CHANNEL
        }
    }

    fn next_task(&mut self) -> Option<CMesg<V>> {
        
        let vec = self.values.atomic_pull();                                   
        if let Some(vec) = vec {
            Some(CMesg::run_task(vec)) 
        } else {
            None
        }

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

        let res = std::thread::scope(            
            |s: &Scope<'_, '_>| {                   
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                                                                                                                                                     
                
                // Generate initial worker threads as you need at least 1 by default. Record control time
                let tm = std::time::Instant::now();   
                for _ in 0..INITIAL_WORKERS {
                    if let Err(e) = self.add_thread(s, &mut threads) {
                        return Err(e);
                    };
                }                                                                                                            
                let mut control_time = tm.elapsed().as_nanos();                          
                                
                let mut task:CMesg<V> = self.next_task().unwrap_or(CMesg::done());                                                            
                           
                let mut process_time = std::time::Instant::now(); 
                let mut elapsed_monitored_time = 0;

                let mut free_threads:VecDeque<usize> = VecDeque::new();

                loop {                                                                                                              
                    if let Ok(msg) = self.all_receiver.try_recv() {
                        if let ThreadMesg::Free(pos, _) = msg {                            
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
                    } 
 
                    if threads.len() < max_threads { 
                        if elapsed_monitored_time as f64 > control_time as f64 {
                            let tm = std::time::Instant::now();
                            if let Err(e) = self.add_thread(s, &mut threads) {
                                return Err(e);
                            };                              
                            control_time = tm.elapsed().as_nanos();                                
                        }                            
                    }                                                                                                                                                                                                                
                }
                
                let mut ignore_threads = vec![false;threads.len()];

                // let tm = std::time::Instant::now(); 
                // let mut inprogress_threads:Vec<usize> = (0..threads.len())
                //                                         .filter_map(|pos| {
                //                                             if threads[pos].primary_q.is_empty() {
                //                                                 None
                //                                             } else {
                //                                                 Some(pos)
                //                                             }
                //                                         }).collect::<Vec<usize>>();
                // println!("in prog list = {}",tm.elapsed().as_micros());  
                // do work stealing and engage free threads and close out all threads
                let tm = std::time::Instant::now(); 
                // println!("Inprogress = {:?}",inprogress_threads);

                let mut task:Option<CMesg<V>> = None;
                loop {                     

                    let mut maxlen:usize = 0;
                    let mut maxpos:isize = -1;
                    if task.is_none() {
                        for (pos,thread) in &mut threads.iter_mut().enumerate() {
                            let currlen = thread.primary_q.len();
                            if currlen > maxlen {
                                maxpos = pos as isize;
                                maxlen = currlen;
                            }                           
                        }

                        if maxpos == -1 { break; }

                        if let Some(pending) = threads[maxpos as usize].primary_q.steal_half() {
                            task = Some(CMesg::run_task(pending));                                                     
                        }
                    }                    

                    if task.is_some() {                        
                        if let Some(free_pos) = free_threads.pop_front() {
                            if free_pos != maxpos as usize {
                                let new_task = task.unwrap();
                                let free_thread = &mut threads[free_pos];                            
                                if let Err(fail_task) = self.send_leaked_task(free_thread, new_task) {
                                    println!("Failed: From:{} To:{}",maxpos,free_pos);
                                    task = Some(fail_task);
                                } else {
                                    println!("Success: From:{} To:{}",maxpos,free_pos);
                                    ignore_threads[free_pos] = true;
                                    task = None;
                                }
                            } else {
                                free_threads.push_back(free_pos);
                            }
                            
                        }
                    } else {
                        break;
                    }                                                            

                    // Get the next free thread
                    while let Ok(msg) = self.all_receiver.try_recv() {
                        if let ThreadMesg::Free(pos, _) = msg {   
                            println!("Freed:{}",pos); 
                            // if !ignore_threads[pos] {
                                free_threads.push_back(pos);  
                            // }                                                                             
                        }
                    }                                                                       
                }                                                   
                println!("work stealing = {}",tm.elapsed().as_micros());    

                //join all threads   
                let tm = std::time::Instant::now();         
                for thread in threads {                                                                    
                    if let Ok(res) = thread.join(){
                        results.extend(res.into_iter());
                    };                                                                                                     
                }  
                println!("time to join = {}",tm.elapsed().as_micros());                    
                       
                Ok(results)
            }
            
        );

        res
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

