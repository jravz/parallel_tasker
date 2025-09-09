//! WorkerThreads enables the launching of thread pool that manages the running of closures
//! based on inputs from the Iterator. This follows a pull phiolosophy and thus threads run till
//! there are Items to pull from the Iterator. Whenever a thread becomes free it pulls a new Item
//! and runs the closure function on the same

use std::{error::Error, fmt::Debug, pin::Pin, sync::{atomic::AtomicPtr, Arc, Mutex}};
use super::iterator::*;

use crate::{collector::Collector, errors::TaskError, parallel_task::ParallelTask, prelude::TaskQueue};

pub struct WorkerThreads {pub nthreads:usize }

#[allow(dead_code)]
impl WorkerThreads
{
    pub fn collect<'a,I,F,T,V,C>(self, mut task:ParallelTask<'a, V,F,T,I>) -> C
    where I:ParallelIter<Item = V> + Send + Sized,
    F: Fn(&V) -> T + Send,
    V: Send,
    T:Send,
    C: Collector<T> {
        let mut vec_handles = Vec::new();                        
        let arc_mut_task: Arc<AtomicPtr<TaskQueue<'a, I, V>>> = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(task.iter))));        
        
        for _ in 0..self.nthreads {
            let builder = std::thread::Builder::new();            
            let result: Result<std::thread::JoinHandle<Vec<T>>, std::io::Error> = unsafe {  
                let arc_mut_task_clone = arc_mut_task.clone();
                let arc_func_clone = *Box::from_raw(Pin::get_unchecked_mut(task.f.as_mut()));
                builder.spawn_unchecked(move ||{Self::task_loop(arc_mut_task_clone, arc_func_clone)})                
            };         
            vec_handles.push(result);
        }

        let mut output = C::initialize();
        for handle in vec_handles {           
            let res =  handle.unwrap().join().unwrap();
            output.extend(res.into_iter());
        }
        output
    }    

    
    /// Task Loop runs the functions within each spawned thread. The Loop runs till the thread is able 
    /// to pop a value from the Iterator. Once there are no more values from the iterator, the loop breaks and
    /// the thread returns all values obtained till that point
    pub fn task_loop<'a,I,F,T,V>(task:Arc<AtomicPtr<TaskQueue<'a, I, V>>>, f:F) -> Vec<T> 
    where I:ParallelIter<Item = V> + Send + Sized,
    F: Fn(&V) -> T + Send,
    V: Send,
    T:Send
    {   
        // let threadid = std::thread::current().id();
        // let mut tot_time = 0;
        // let mut waiting_time = 0;        
        let mut res = Vec::new();  
        // let ttm = std::time::Instant::now();      
        while let Some(input) = {
            // let tm = std::time::Instant::now();
            let val = if let Some(task_mutex) = unsafe { task.load(std::sync::atomic::Ordering::Acquire).as_mut()} 
            {
                task_mutex.pop()
            } else {
                None
            };
            // waiting_time += tm.elapsed().as_micros();
            val
        } 
        {           
            let result = f(input);                         
            res.push(result);                     
        }                
        // tot_time = ttm.elapsed().as_micros();
        // println!("ID: {:?} -> total time = {} ms, waiting time = {}",threadid, tot_time, waiting_time);
        res
    }
}


