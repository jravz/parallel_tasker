//! WorkerThreads enables the launching of thread pool that manages the running of closures
//! based on inputs from the Iterator. This follows a pull phiolosophy and thus threads run till
//! there are Items to pull from the Iterator. Whenever a thread becomes free it pulls a new Item
//! and runs the closure function on the same

use std::{error::Error, pin::Pin, sync::{Arc, Mutex}};

use crate::{collector::Collector, errors::TaskError, parallel_task::Tasks};

pub struct WorkerThreads {pub nthreads:usize }

#[allow(dead_code)]
impl WorkerThreads
{
    pub fn collect<I,F,T,V,C>(self, task:Tasks<I,V,F,T>) -> C
    where I: Iterator<Item = V> + Send, 
    F: Fn(V) -> T + Send,
    T: Send,
    V: Send,
    C: Collector<T> {
        let mut vec_handles = Vec::new();
        let arc_mut_task = Arc::new(Mutex::new(task));            

        for _ in 0..self.nthreads {
            let builder = std::thread::Builder::new();
            let arc_mut_task_clone = arc_mut_task.clone();   
            let result: Result<std::thread::JoinHandle<Vec<T>>, std::io::Error> = unsafe {  
                
                let res_run_func =  if let Ok(mut task_mutex) = arc_mut_task_clone.lock() {
                    let result = *Box::from_raw(Pin::get_unchecked_mut(task_mutex.f.as_mut()));                   
                    drop(task_mutex); 
                    Ok(result)
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Valid function missing!"))
                };
                if let Ok(run_func) = res_run_func {
                    builder.spawn_unchecked(move ||{Self::task_loop(arc_mut_task_clone, run_func)})
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Valid function missing!"))
                } 

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

    pub fn try_collect<I,F,T,V,C>(self, task:Arc<Mutex<Tasks<I,V,F,T>>>) -> Result<C,Box<dyn Error>>
    where I: Iterator<Item = V> + Send, 
    F: Fn(V) -> T + Send,
    T: Send,
    V: Send,
    C: Collector<T> 
    {
        let mut vec_handles = Vec::new();          

        for _ in 0..self.nthreads {
            let builder = std::thread::Builder::new();
            let arc_mut_task_clone = task.clone();   
            let result: Result<std::thread::JoinHandle<Vec<T>>, std::io::Error> = unsafe {  
                
                let res_run_func =  if let Ok(mut task_mutex) = task.lock() {
                    let result = *Box::from_raw(Pin::get_unchecked_mut(task_mutex.f.as_mut()));                   
                    drop(task_mutex); 
                    Ok(result)
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Valid function missing!"))
                };
                if let Ok(run_func) = res_run_func {
                    builder.spawn_unchecked(move ||{Self::task_loop(arc_mut_task_clone, run_func)})
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Valid function missing!"))
                } 

            };         
            vec_handles.push(result);
        }

        let mut output = C::initialize();
        for handle in vec_handles {           
            let res= handle?
                            .join()
                            .map_err(|e| {
                                if let Some(err) = e.downcast_ref::<String>() {
                                    Box::new(TaskError::Other(err.to_owned()))
                                } else {
                                    Box::new(TaskError::ThreadJoin)
                                }   
                            })?;
            output.extend(res.into_iter());
        }
        Ok(output)
    }

    /// Task Loop runs the functions within each spawned thread. The Loop runs till the thread is able 
    /// to pop a value from the Iterator. Once there are no more values from the iterator, the loop breaks and
    /// the thread returns all values obtained till that point
    pub fn task_loop<I,F,T,V>(task:Arc<Mutex<Tasks<I,V,F,T>>>, f:F) -> Vec<T> 
    where I: Iterator<Item = V> + Send, 
    F: Fn(V) -> T + Send,
    T: Send,
    V: Send
    {    
        let mut res = Vec::new();
        while let Ok(mut task_mutex) = task.lock() {            
            if let Some(input )= task_mutex.pop() {
                drop(task_mutex);                    
                let result = f(input);            
                res.push(result);
            } else {
                break;
            }                    
        }                
        
        res
    }
}


