const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

use std::marker::PhantomData;
use std::pin::Pin;
use crate::worker_thread::WorkerThreads;
use super::iterator::*;

use super::collector::*;

/// ParallelTaskIter is an experiment to create a simple module to help manage CPU intensive jobs across threads. This proposes
/// that a work stealing algorithm is not always necessary and a simple pull (.next) based approach can be equally effective in specific use case.
/// ```
/// use parallel_task::prelude::*;
/// 
/// let res = (0..100_000).collect::<Vec<i32>>().parallel_task(|val|*val).collect::<Vec<i32>>();
/// assert_eq!(res.len(),100_000)
/// ```
/// 
pub trait ParallelTaskIter<'a, I, V,F,T>
where Self: ParallelIter<Item = V> + Send + Sized,
F: Fn(&V) -> T + Send,
V: Send,
T:Send 
{
    fn parallel_task(&'a self,f:F) -> ParallelTask<'a, V,F,T,Self>{
        ParallelTask::new(self,f)
    }
}

impl<'a,I,V,F,T> ParallelTaskIter<'a, I, V,F,T> for I 
where I: ParallelIter<Item = V> + Send + Sized,
F: Fn(&V) -> T + Send,
V: Send,
T:Send {}

/// Tasks is a structure type that captures the information necessary to run the values within the Iterator in parallel
/// Its the result of parallel_task that can be run on any Iterator implementing type.
pub struct ParallelTask<'a, V,F,T,I>
where  I:ParallelIter<Item = V> + Send + Sized,
F: Fn(&V) -> T + Send,
V: Send,
T:Send
{
    pub iter: TaskQueue<'a,I,V>,
    pub f:Pin<Box<F>>,
    pub num_threads:usize,
    pub v: PhantomData<V>,
    pub t: PhantomData<T>,
}

pub struct TaskQueue<'a,I,V> 
where I:ParallelIter<Item = V> + Send + Sized,
V: Send
{
    pub iter: ParallelIterator<'a,I>
}

impl<'a,I,V> TaskQueue<'a,I,V> 
where I:ParallelIter<Item = V> + Send + Sized,
V:Send
{
    pub fn pop(&mut self) -> Option<&V> {
        self.iter.atomic_next()
    }
}

#[allow(dead_code)]
impl<'a, I,V,F,T> ParallelTask<'a, V,F,T,I>
where I:ParallelIter<Item = V> + Send + Sized,
F: Fn(&V) -> T + Send,
V: Send,
T:Send
{
    pub fn new(iter:&'a I,f:F) -> Self
    {                   
        Self {
            iter: TaskQueue { iter: iter.parallel_iter() },
            f:Box::pin(f),
            num_threads: Self::max_threads(),
            v: PhantomData::default(),
            t: PhantomData::default()    
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
        
    pub fn collect<C>(self) -> C
    where C: Collector<T>
    {                
        let num_threads = self.num_threads;        

        WorkerThreads { nthreads: num_threads }
        .collect(self)      
        
    }
}

