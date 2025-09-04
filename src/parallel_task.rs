const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

use std::pin::Pin;
use crate::worker_thread::WorkerThreads;

use super::collector::*;

/// ParallelTaskIter is an experiment to create a simple module to help manage CPU intensive jobs across threads. This proposes
/// that a work stealing algorithm is not always necessary and a simple pull (.next) based approach can be equally effective in specific use case.
/// ```
/// use parallel_task::prelude::*;
/// 
/// let res = (0..100_000).parallel_task(|val|val).collect::<Vec<i32>>();
/// assert_eq!(res.len(),100_000)
/// ```
/// 
pub trait ParallelTaskIter<V,F,T> 
where Self: Iterator<Item = V> + Send + Sized, 
F: Fn(V) -> T + Send,
V: Send,
T:Send
{
    fn parallel_task(self,f:F) -> ParallelTask<Self,V,F,T> {
        ParallelTask::new(self,f)
    }
}

impl<I,V,F,T> ParallelTaskIter<V,F,T> for I 
where I: Iterator<Item = V> + Send, 
F: Fn(V) -> T + Send,
V: Send,
T:Send {}

/// Tasks is a structure type that captures the information necessary to run the values within the Iterator in parallel
/// Its the result of parallel_task that can be run on any Iterator implementing type.
pub struct ParallelTask<I,V,F,T>
where I: Iterator<Item = V> + Send, 
F: Fn(V) -> T + Send,
V: Send,
T:Send
{
    pub iter: I,
    pub f:Pin<Box<F>>,
    pub num_threads:usize
}

#[allow(dead_code)]
impl<I,V,F,T> ParallelTask<I,V,F,T> 
where I: Iterator<Item = V> + Send, 
F: Fn(V) -> T + Send,
V: Send,
T:Send
{
    pub fn new(iter:I,f:F) -> Self {                   
        Self {
            iter,
            f:Box::pin(f),
            num_threads: Self::max_threads()    
        }
    }

    fn max_threads() -> usize {
        // available_parallelism() function gives an idea of the CPUs available 
        let num_threads:usize = if let Ok(available_cpus) = std::thread::available_parallelism() {
            available_cpus.get()
        } else {
            DEFAULT_THREADS_NUM
        } * (CPU_2_THREAD_RATIO);

        num_threads
    }

    /// Set the thread pool size for running the parallel tasker    
    pub fn threads(mut self, nthreads:usize) -> Self {
        self.num_threads = usize::min(Self::max_threads(),nthreads);
        self
    }

    pub fn pop(&mut self) -> Option<V> {
        self.iter.next()
    }
        
    pub fn collect<C>(self) -> C
    where C: Collector<T>
    {                
        let num_threads = self.num_threads;        

        WorkerThreads { nthreads: num_threads }
        .collect(self)      
        
    }
}

