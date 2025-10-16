use std::sync::{Arc, RwLock};
use std::thread::Scope;
use crate::collector::Collector;
use crate::errors::WorkThreadError;
use crate::prelude::AtomicIterator;
use crate::push_workers::thread_manager::ThreadManager;

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
    avg_task_len: Option<usize>,    
    max_threads: usize,
}

impl<F,V,T,I>  WorkerController<F,V,T,I>
where F: Fn(V) -> T + Send + Sync,
V: Send + Sync,
T: Send + Sync,    
I:AtomicIterator<AtomicItem = V> + Send + Sized  
{

    pub fn new(f:F, values:I) -> Self 
    {                      
        Self {
            f: Arc::new(RwLock::new(f)),
            values,            
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
                let mut thread_manager = ThreadManager::new(s,self.f.clone(), self.max_threads);                                                                                                                                                                                                                                                                                                                                                                                                                                     
                let control_time = self.primary_queue_distribution(s, &mut thread_manager)?;                    
                self.redistribute_among_threads( &mut thread_manager,control_time);                                              
                thread_manager.join_all_threads()
            }            
        )        
    }

    fn primary_queue_distribution<'env, 'scope>(&mut self, s: &'scope std::thread::Scope<'scope, 'env>, 
    thread_manager: &mut ThreadManager<'env, 'scope,V,T,F>) -> Result<u128,WorkThreadError>
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
            _ = thread_manager.add_thread();           
        });
        let control_time = tm.elapsed().as_nanos() / INITIAL_WORKERS as u128; 
         
        (0..INITIAL_WORKERS).for_each(|pos| {
            let thread = thread_manager.get_mut_thread(pos);
            if self.send_task(thread,vec_tasks.pop()).is_err() {  
                thread_manager.add_to_free_queue(pos);                                  
            };
        });

        Ok(control_time)
    }

    

    //redistribution works on the principle that if there is a free thread and there is another thread that has a large
    //queue of tasks, then the former should get half to save on time.
    //It does this till the thread with the biggest queue has upto or less than 10% of the tasks from the intial 
    //chunkwise distribution in the primary loop. 
    fn redistribute_among_threads<'env, 'scope>(&mut self, thread_manager: &mut ThreadManager<'env, 'scope,V,T,F>, mut control_time:u128) 
    where 'env: 'scope,     
    V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,   
    F: Send + Sync + 'scope,   
    {
        let tm = std::time::Instant::now();
        if thread_manager.thread_len() > 0 //if just 2 threads, there is nothing to redistribute as such
        && self.avg_task_length().is_some() //ensure at least one set of values was sent to queue
        {                                                                               
            let mut stop_loop = false;
            loop {                 
                if thread_manager.has_free_threads() {                     
                    // println!("1: {:?}",free_threads);                                                                 
                    if let Ok(tm) = thread_manager.refresh_free_threads(control_time) {
                        control_time = tm;
                    }
                    // println!("2: {:?}",free_threads);
                }

                while !thread_manager.has_free_threads() && !stop_loop {
                    // println!("2");
                    // println!("3: {:?}",free_threads);
                    let mut vec_ranking:Vec<(usize, usize)> = Vec::new();
                    for idx in 0..thread_manager.thread_len() {
                        let thread = thread_manager.get_mut_thread(idx);
                        // print!("{}:{} ",thread.pos(),thread.queue_len());
                        vec_ranking.push((thread.pos(),thread.queue_len()));
                    }
                    // println!("... {} ns",tm.elapsed().as_nanos());

                    let mut task:Option<Vec<V>>;
                    vec_ranking.sort_by(|a,b|b.1.cmp(&a.1));
                    let min_allowed_length = self.avg_task_length().unwrap() * PERCENT_OF_INITIAL_CHUNK / 100 ;
                    // println!("tm:{} min_allowed_length = {}, free_threads = [{:?}]",tm.elapsed().as_nanos(),min_allowed_length,free_threads);
                    for (idx,(pos,remaining))  in vec_ranking.into_iter().enumerate() {                        
                        if remaining <= min_allowed_length {                                    
                            if idx == 0 {                                        
                                stop_loop = true;
                                break;
                            }
                        } 

                        if let Some(freepos) = thread_manager.pop_from_free_queue() {                                                                     
                            task =thread_manager.get_mut_thread(freepos).primary_q.steal_half(); 
                            if let Some(new_task) = task {                                        
                                if new_task.is_empty() {                                            
                                    thread_manager.add_to_free_queue(freepos);
                                } else {                                            
                                    let free_thread = thread_manager.get_mut_thread(freepos);
                                    if self.send_leaked_task(free_thread, new_task).is_err() {
                                        stop_loop = true;                         
                                        break;
                                    };
                                }
                            } else {                                        
                                thread_manager.add_to_free_queue(freepos);
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

