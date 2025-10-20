//! WorkerThreads follow a Push based strategy wherein Threads are managed by a WorkerController that
//! spawns WorkerThreads. These worker threads can be communicated with via sync and async channels to 
//! send data for processing and to close the same

use crate::{collector::Collector, errors::WorkThreadError, for_each::ParallelForEach, iterators::iterator::AtomicIterator, map::ParallelMap, push_workers::{priorisation::ThreadPrioritization, worker_controller::WorkerController}};
pub struct WorkerThreads {pub nthreads:usize }

#[allow(dead_code)]
impl WorkerThreads
{
    pub fn collect<I,F,T,V,C>(self, task:ParallelMap<V,F,T,I>) -> C
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V) -> T + Send + Sync,
    V: Send + Sync,
    T:Send + Sync,
    C: Collector<T> {          
        let fnc = task.f;       
        let q = task.iter.iter;        
        match WorkerController::new(fnc,q, ThreadPrioritization::Remaining)
        .run::<C>() {
            Ok(res) => { res }
            Err(e) => {
                if let WorkThreadError::ThreadAdd(e) = e {
                    panic!("Error: {}",e);
                } else {
                    panic!("Unknown error occurred in worker controller");
                }
            }
        }               
    }  

    pub fn run<I,F,V>(self, task:ParallelForEach<V,F,I>)
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V) + Send + Sync,
    V: Send + Sync,    
    {
        let fnc = task.f;       
        let q = task.iter.iter;            

        if let Err(e) = WorkerController::new(fnc,q,ThreadPrioritization::Remaining)
         .run::<Vec<_>>() {            
            if let WorkThreadError::ThreadAdd(e) = e {
                panic!("Error: {}",e);
            } else {
                panic!("Unknown error occurred in worker controller");
            }        
        };             
    }    
}


