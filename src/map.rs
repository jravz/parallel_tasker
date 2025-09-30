//! ParallelMap is a structure type that captures the Map object and function necessary to run the values within the AtomicIterator in parallel.
const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

use std::marker::PhantomData;
use crate::task_queue::TaskQueue;
use crate::worker_thread::WorkerThreads;
use super::iterators::iterator::*;

use super::collector::*;

/// ParallelMapIter allows calling the .map(f) on type implementing AtomicIterator to get a ParallelMap object on which collect may be called.
/// ```
/// use parallel_task::prelude::*;
/// 
/// let res = (0..100_000).collect::<Vec<i32>>().parallel_iter().map(|val|*val).collect::<Vec<i32>>();
/// assert_eq!(res.len(),100_000)
/// ```
/// 
pub trait ParallelMapIter<I, V,F,T>
where Self: AtomicIterator<AtomicItem = V> + Send + Sized,
F: Fn(V) -> T + Send + Sync,
V: Send,
T:Send 
{
    fn map(self,f:F) -> ParallelMap<V,F,T,Self>{
        ParallelMap::new(self,f)
    }
}

impl<I,V,F,T> ParallelMapIter<I, V,F,T> for I 
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: Fn(V) -> T + Send + Sync,
V: Send,
T:Send {}

pub struct ParallelMap<V,F,T,I>
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: Fn(V) -> T + Send + Sync,
V: Send,
T:Send 
{
    pub iter: TaskQueue<I,V>,
    pub f:F,
    pub num_threads:usize,
    pub v: PhantomData<V>,
    pub t: PhantomData<T>,
}

/// ParallelMap is a object that allows map function to be run on AtomicIterators.
/// The results may be then collected in types implementing Collector trait.
/// 
/// ```
/// use parallel_task::prelude::*;
/// 
/// let res = (0..100_000).collect::<Vec<i32>>().parallel_iter().map(|val|*val).collect::<Vec<i32>>();
/// assert_eq!(res.len(),100_000)
/// ```
/// 
#[allow(dead_code)]
impl<I,V,F,T> ParallelMap<V,F,T,I>
where I:AtomicIterator<AtomicItem = V> + Send + Sized,
F: Fn(V) -> T + Send + Sync,
V: Send,
T:Send
{
    pub fn new(iter:I,f:F) -> Self
    {                   
        Self {
            iter: TaskQueue { iter },
            f,
            num_threads: Self::max_threads(),
            v: PhantomData,
            t: PhantomData    
        }
    }

    /// Available_parallelism() function gives an idea of the CPUs available. 
    fn max_threads() -> usize {        
        let num_threads:usize = if let Ok(available_cpus) = std::thread::available_parallelism() {
            available_cpus.get()
        } else {
            DEFAULT_THREADS_NUM
        } * (CPU_2_THREAD_RATIO);

        num_threads
    }

    /// Set the thread pool size for running the parallel tasker.    
    pub fn threads(mut self, nthreads:usize) -> Self {
        self.num_threads = usize::min(Self::max_threads(),nthreads);
        self
    }

    /// Collect the results of the Map in a type implementing Collector trait    
    pub fn collect<C>(self) -> C
    where C: Collector<T>
    {                
        let num_threads = self.num_threads;        

        WorkerThreads { nthreads: num_threads }
        .collect(self)      
        
    }
}

