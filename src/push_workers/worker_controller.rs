use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel, Receiver, Sender};
use crate::collector::Collector;
use crate::errors::WorkThreadError;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::WorkerThread;

pub const INITIAL_WORKERS:usize = 2;
const PERCENT_OF_INITIAL_CHUNK:usize = 1;

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
    avg_task_len: Option<usize>,
    max_threads: usize,
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
        //send/receive channel for each thread's task runner to apply for the next job
        let (all_sender, all_receiver) = async_channel::<ThreadMesg>();        
        Self {
            f: Arc::new(RwLock::new(f)),
            values,
            all_sender,
            all_receiver,            
            avg_task_len:None,
            max_threads: crate::utils::max_threads()/2
        }
    }

    pub fn set_max_threads(&mut self, limit:usize) {
        self.max_threads = usize::min(limit, crate::utils::max_threads());
    }

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
        match WorkerThread::launch(scope,worker_sender_clone,threads.len(), arc_f_clone) {
            Ok(t) =>  {                                                    
                threads.push(t);  
                Ok(())                                                                                                        
            } 
            Err(e) =>  {
                Err(WorkThreadError::ThreadAdd(e.to_string()))
            }
        }                              
    }    

    fn add_next_task(&mut self, vec_tasks:&mut Vec<Vec<V>>) {
        if let Some(vec) = self.next_task() {
            vec_tasks.push(vec);
        }
    }

    ///run function is usually called after WorkerController is instantiated.
    ///It is responsible for running the three processes: generate threads and pull from primary queue, 
    /// redistribute and conquer work amongst threads and join for closure
    pub fn run<C>(&mut self) -> Result<C,WorkThreadError>
    where C: Collector<T>,    
    {                                             
        std::thread::scope(            
            |s: &Scope<'_, '_>| {                                                          
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                                                                                                                                                                                                                                                                                                                                        
                let mut free_threads:VecDeque<usize> = VecDeque::new();                
                let control_time = self.primary_queue_distribution(s, &mut free_threads, &mut threads)?;                                                  
                self.redistribute_among_threads(s, &mut free_threads, &mut threads,control_time);                                  
                self.join_all_threads(threads)
            }            
        )        
    }

    fn join_all_threads<'env, 'scope,C>(&mut self, threads:Vec<WorkerThread<'scope, V,T>>) -> Result<C,WorkThreadError>
    where 'env: 'scope,     
    V: Send + Sync + 'scope,
    T: Send + Sync + 'scope, 
    F:Fn(V) -> T + Send + Sync + 'scope,
    C: Collector<T>   
    {
        let mut results = C::initialize();   
        for thread in threads {
            results.extend(
                thread.join()
                .map_err(|e|WorkThreadError::ThreadJoin)?
                .into_iter()
            );
        }        

        Ok(results)

    }

    fn primary_queue_distribution<'env, 'scope>(&mut self, s: &'scope std::thread::Scope<'scope, 'env>, free_threads:&mut VecDeque<usize>, 
    mut threads:&mut Vec<WorkerThread<'scope, V,T>>) -> Result<u128,WorkThreadError>
    where 'env: 'scope,     
    V: Send + Sync + 'scope,
    T: Send + Sync + 'scope, 
    F:Fn(V) -> T + Send + Sync + 'scope
    {

        // Intermediate buffer to store the tasks
        let mut vec_tasks:Vec<Vec<V>> = Vec::new();        

        // Generate initial worker threads as you need at least 1 by default. Record control time
        let tm = std::time::Instant::now();                   
        (0..INITIAL_WORKERS).for_each(|_| {
            self.add_next_task(&mut vec_tasks);
            _ = self.add_thread(s, &mut threads);           
        });
        let control_time = tm.elapsed().as_nanos() / INITIAL_WORKERS as u128; 
         
        (0..INITIAL_WORKERS).for_each(|pos| {
            let thread = &mut threads[pos];
            if self.send_task(thread,vec_tasks.pop()).is_err() {  
                free_threads.push_front(pos);                                              
            };
        });

        // let mut process_time = std::time::Instant::now();                                           
        
        // // Start the primary process loop to clear the primary queue
        // loop {                                           

        //     if let Ok(ThreadMesg::Free(pos,_)) = self.all_receiver.try_recv() {
        //         process_time = std::time::Instant::now();                                                     
        //         let thread= &mut threads[pos]; 

        //         //When primary queue is completed, break from the same. 
        //         if self.send_task(thread,vec_tasks.pop()).is_err() {  
        //             free_threads.push_front(pos);                          
        //             break;
        //         };
        //     }                                                                                                                                                                 
        //     // Keep pulling in at every opportunity to save on time                    
        //     self.add_next_task(&mut vec_tasks);

        //     if threads.len() < self.max_threads && 
        //     process_time.elapsed().as_nanos() as f64 > control_time as f64 &&
        //     self.no_thread_empty(&mut threads)
        //     {                         
        //         process_time = std::time::Instant::now();                   
        //         let tm = std::time::Instant::now();
        //         self.add_thread(s, &mut threads)?;                                                    
        //         control_time = tm.elapsed().as_nanos();                                
        //     }                                                                                                                                                                                                                                                           
        // }
        Ok(control_time)
    }

    fn refresh_free_threads<'scope,'env>(&mut self, s: &'scope std::thread::Scope<'scope, 'env>, free_threads:&mut VecDeque<usize>, mut threads:&mut Vec<WorkerThread<'scope, V,T>>, 
    mut control_time:u128) -> Result<u128,WorkThreadError>
    where 'env: 'scope,     
    V: Send + Sync + 'scope,
    T: Send + Sync + 'scope, 
    F: Send + Sync + 'scope, 
    {
        
        free_threads.clear();
        let mut is_process_time_high = false;
        for idx in 0..threads.len() {
            let thread = &mut threads[idx];
            if !thread.is_running() && thread.primary_q.is_empty() {
                free_threads.push_back(idx);
            } else {
                if thread.get_elapsed_time() > control_time {
                    is_process_time_high = true;
                }
            }
        }

        if free_threads.is_empty() && is_process_time_high && threads.len() < self.max_threads {
            let tm = std::time::Instant::now();
            self.add_thread(s, &mut threads)?;                                                    
            control_time = tm.elapsed().as_nanos(); 
            free_threads.push_back(threads.len() - 1);
        } 
        Ok(control_time)
    }

    //redistribution works on the principle that if there is a free thread and there is another thread that has a large
    //queue of tasks, then the former should get half to save on time.
    //It does this till the thread with the biggest queue has upto or less than 10% of the tasks from the intial 
    //chunkwise distribution in the primary loop. 
    fn redistribute_among_threads<'env, 'scope>(&mut self, s: &'scope std::thread::Scope<'scope, 'env>, free_threads:&mut VecDeque<usize>, threads:&mut Vec<WorkerThread<'scope, V,T>>, mut control_time:u128) 
    where 'env: 'scope,     
    V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,   
    F: Send + Sync + 'scope,   
    {
        // let tm = std::time::Instant::now();
        if threads.len() > 0 //if just 2 threads, there is nothing to redistribute as such
        && self.avg_task_length().is_some() //ensure at least one set of values was sent to queue
        {                                                                               
            let mut stop_loop = false;
            loop { 
                if free_threads.is_empty() {   
                    // println!("1: {:?}",free_threads);                                                                 
                    if let Ok(tm) = self.refresh_free_threads(s, free_threads,threads, control_time) {
                        control_time = tm;
                    }
                    // println!("2: {:?}",free_threads);
                }

                while !free_threads.is_empty() && !stop_loop {
                    // println!("3: {:?}",free_threads);
                    let mut vec_ranking:Vec<(usize, usize)> = Vec::new();
                    for idx in 0..threads.len() {
                        let thread = &mut threads[idx];
                        // print!("{}:{} ",thread.pos(),thread.queue_len());
                        vec_ranking.push((thread.pos(),thread.queue_len()));
                    }
                    // println!("... {} ns",tm.elapsed().as_nanos());

                    let mut task:Option<Vec<V>>;
                    vec_ranking.sort_by(|a,b|b.1.cmp(&a.1));
                    let min_allowed_length = self.avg_task_length().unwrap() * PERCENT_OF_INITIAL_CHUNK / 100 ;
                    for (idx,(pos,remaining))  in vec_ranking.into_iter().enumerate() {                        
                        if remaining < min_allowed_length {                                    
                            if idx == 0 {                                        
                                stop_loop = true;
                                break;
                            }
                        } 

                        if let Some(freepos) = free_threads.pop_front() {                                                                     
                            task = threads[pos as usize].primary_q.steal_half(); 
                            if let Some(new_task) = task {                                        
                                if new_task.is_empty() {                                            
                                    free_threads.push_back(freepos);
                                } else {                                            
                                    let free_thread = &mut threads[freepos];
                                    if self.send_leaked_task(free_thread, new_task).is_err() {
                                        stop_loop = true;                         
                                        break;
                                    };
                                }
                            } else {                                        
                                free_threads.push_back(freepos);
                            } 
                                
                        } else {                                    
                            break;
                        }
                    }                   
                }
                
                if stop_loop {                            
                    break;
                } 
            }                    
        } 
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
        thread.run(values)                             
    }
    

    fn send_task<'scope>(&mut self, thread:&mut WorkerThread<'scope, V,T>, task:Option<Vec<V>>) -> Result<(),()>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {            
        if let Some(values) = task {
            if self.avg_task_length().is_none() {
                self.set_avg_task_length(values.len());
            }                          
            thread.run(values)
        } else {            
            Err(()) 
        }                   
    }
}

