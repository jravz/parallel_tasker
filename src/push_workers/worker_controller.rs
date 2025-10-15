use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel, Receiver, Sender};
use crate::collector::Collector;
use crate::errors::WorkThreadError;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::{Coordination, WorkerThread};

// With 2 queues, the thread always has a backup to work on and does not wait
const BUF_SIZE_CHANNEL:usize = 1;
const INITIAL_WORKERS:usize = 1;

//function to read the iterator in parallel
// struct ParallelQueueCreate<I,V> 
// where I:AtomicIterator<AtomicItem=V>
// {
//     obj: ReadAccessor<Vec<V>>,
//     iterator:AtomicIterIngestor<I>
// }

// impl<V> ParallelQueueCreate<V> {

//     pub fn new(iter:I) -> 

// }

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
    buf_size: usize,
    avg_task_len: Option<usize>
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
            buf_size: BUF_SIZE_CHANNEL,
            avg_task_len:None
        }
    }

    // fn next_task(&mut self) -> Option<CMesg<V>> {
        
    //     let opt_vec = self.values.atomic_pull(); 
    //     opt_vec.map(CMesg::run_task)                                          
    // }

    fn avg_task_length(&self) -> Option<usize> {
        self.avg_task_len
    }

    fn set_avg_task_length(&mut self, size:usize) {
        self.avg_task_len = Some(size);
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

    fn refresh_free_threads<'scope>(threads:&mut Vec<WorkerThread<'scope, V,T>>, free_threads:&mut VecDeque<usize>) {
        free_threads.clear();        
        for thread in threads {
            if thread.primary_q.is_empty() {
                free_threads.push_back(thread.pos());
            }
        }
    }

    fn prerefresh_free_threads<'scope>(threads:&mut Vec<WorkerThread<'scope, V,T>>, free_threads:&mut VecDeque<usize>,tasklen:usize) {
        free_threads.clear();
        println!("tasklen = {}",tasklen);
        for thread in threads {
            if thread.primary_q.len() < (25/100 * tasklen) {
                free_threads.push_back(thread.pos());
            }
        }
    }

    fn add_next_task(&mut self, vec_tasks:&mut Vec<Vec<V>>) {
        if let Some(vec) = self.next_task() {
            vec_tasks.push(vec);
        }
    }

    pub fn run<C>(&mut self) -> Result<C,WorkThreadError>
    where C: Collector<T>,    
    {                        
        let max_threads = crate::utils::max_threads();              

        std::thread::scope(            
            |s: &Scope<'_, '_>| {                  
                let mut vec_tasks:Vec<Vec<V>> = Vec::new();
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                                                                                                                                                     
                                                                                                                                                                                   
                let mut free_threads:VecDeque<usize> = VecDeque::new();  
                self.add_next_task(&mut vec_tasks);

                // Generate initial worker threads as you need at least 1 by default. Record control time
                let tm = std::time::Instant::now();   
                for idx in 0..INITIAL_WORKERS {
                    self.add_thread(s, &mut threads)?;
                    let thread = &mut threads[idx];
                    self.send_task(thread,vec_tasks.pop()); 
                    self.add_next_task(&mut vec_tasks);
                }
                let mut control_time = tm.elapsed().as_nanos() / INITIAL_WORKERS as u128;   
                let mut process_time = std::time::Instant::now();                                           
                loop {                                           

                    if let Some(pos) = free_threads.pop_front() {
                        process_time = std::time::Instant::now();                                                     
                        let thread= &mut threads[pos]; 

                        //When primary queue is completed, break from the same. 
                        if self.send_task(thread,vec_tasks.pop()).is_none() {                            
                            break;
                        };                        
                    } else {
                        // Self::prerefresh_free_threads(&mut threads, &mut free_threads,self.avg_task_length().unwrap());
                        Self::refresh_free_threads(&mut threads, &mut free_threads);
                    }                                                                                                                                                  
                    
                    self.add_next_task(&mut vec_tasks);

                    if threads.len() < max_threads && 
                    process_time.elapsed().as_nanos() as f64 > control_time as f64 &&
                    self.no_thread_empty(&mut threads)
                    {                         
                        process_time = std::time::Instant::now();                   
                        let tm = std::time::Instant::now();
                        self.add_thread(s, &mut threads)?;                                                    
                        control_time = tm.elapsed().as_nanos();                                
                    }                                                                                                                                                                                                                                                           
                }
                                                    
                if threads.len() > 2 {                    
                    let mut task:Option<Vec<V>> = None;
                    let mut min_rate_change:f64;
                    let mut maxpos:isize=-1;
                    let mut jobstatus = (0..threads.len())
                    .map(|idx| (threads[idx].primary_q.len(),0.0))
                    .collect::<Vec<(usize,f64)>>();
                    loop {                                         
                        if task.is_none() {                            
                            min_rate_change = 1.0;
                            maxpos = -1;
                            let mut maxlen:usize = self.avg_task_length().unwrap_or(100usize) * 10 / 100; // No point if there are only two tasks
                            let mut rate_change_array:Vec<f64> = Vec::new();
                            for (pos,thread) in &mut threads.iter_mut().enumerate() {                                
                                let currlen = thread.primary_q.len();
                                let (lastlen, _rate_of_change) = jobstatus[pos];                            
                                let curr_rate_change = if (lastlen == 0) || (lastlen < currlen) { 1.0 } else { (lastlen - currlen) as f64 / lastlen as f64 };                                                        
                                jobstatus[pos] = (currlen,curr_rate_change);
                                rate_change_array.push(curr_rate_change);                                
                                if curr_rate_change < min_rate_change                                   
                                &&  currlen > maxlen {                                    
                                    maxpos = pos as isize;
                                    min_rate_change = curr_rate_change;
                                    maxlen = currlen;
                                }                           
                            }                                                       

                            if maxpos == -1 {                                 
                                break; 
                            }

                            task = threads[maxpos as usize].primary_q.steal_half();                            
                        }                    

                        if task.is_some() {                                                                                                     
                            if let Some(free_pos) = free_threads.pop_front() {                                                                                               
                                let new_task = task.unwrap();                                
                                let free_thread = &mut threads[free_pos];                            
                                if self.send_leaked_task(free_thread, new_task).is_err() {
                                    break;
                                };  
                                task = None;                                                                                                                                                 
                            } else {                                
                                // wait for the next free thread
                            }
                        } else {
                            break;
                        }                                                            
                        
                        if free_threads.is_empty() {
                            Self::refresh_free_threads(&mut threads, &mut free_threads);                                                                                          
                        }                        
                    }     
                }                                                                                                                 
                                
                //join all threads                     
                for thread in threads {   
                    // println!("pos:{}, remaining:{}",thread.pos(), thread.primary_q.len());                                                                                    
                    if let Ok(res) = thread.join(){
                        results.extend(res.into_iter());
                    };                                                                                                     
                }                                               
                                
                Ok(results)
            }
            
        )        
    }

    fn no_thread_empty<'scope>(&mut self, threads:&mut Vec<WorkerThread<'scope, V,T>>) -> bool {
        
        for thread in threads {
            if thread.primary_q.is_empty() {
                return false;
            }
        }
        true
    }

    fn next_task(&mut self) -> Option<Vec<V>> {
        self.values.atomic_pull()
    }

    fn send_leaked_task<'scope>(&mut self, thread:&mut WorkerThread<'scope, V,T>, values:Vec<V>) -> Result<(),()>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {                 
        if values.len() > 0 {
            if thread.primary_q.replace(values).is_ok() {
                thread.signal(Coordination::Run);
                return Ok(());
            }
            Err(())
        } else {
            Err(())
        }    
               
    }
    

    fn send_task<'scope>(&mut self, thread:&mut WorkerThread<'scope, V,T>, task:Option<Vec<V>>) -> Option<()>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {             
        if let Some(values) = task {

            if self.avg_task_length().is_none() {
                self.set_avg_task_length(values.len());
            }

            if thread.primary_q.replace(values).is_ok() {
                thread.signal(Coordination::Run);
                return Some(());
            }             
        }         
        None            
    }
}

