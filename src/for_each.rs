const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

use std::marker::PhantomData;
use std::pin::Pin;
use crate::task_queue::TaskQueue;
use crate::worker_thread::WorkerThreads;
use super::iterators::iterator::AtomicIterator;

/// ParallelTaskIter is an experiment to create a simple module to help manage CPU intensive jobs across threads. This proposes
/// that a work stealing algorithm is not always necessary and a simple pull (.next) based approach can be equally effective in specific use case.
/// ```
/// use parallel_task::prelude::*;
/// 
/// let res = (0..100_000).collect::<Vec<i32>>().parallel_iter().parallel_task(|val|*val).collect::<Vec<i32>>();
/// assert_eq!(res.len(),100_000)
/// ```
/// 
pub trait ParallelForEachIter<I, V,F>
where Self: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send
{
    fn for_each(self,f:F) {
        ParallelForEach::new(self,f).run()
    }
}

impl<I,V,F> ParallelForEachIter<I, V,F> for I 
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send,
{}

/// Tasks is a structure type that captures the information necessary to run the values within the Iterator in parallel
/// Its the result of parallel_task that can be run on any Iterator implementing type.
pub struct ParallelForEach<V,F,I>
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V),
V: Send
{
    pub iter: TaskQueue<I,V>,
    pub f:Pin<Box<F>>,
    pub num_threads:usize,
    pub v: PhantomData<V>,    
}

#[allow(dead_code)]
impl<I,V,F> ParallelForEach<V,F,I>
where I:AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send,
{
    pub fn new(iter:I,f:F) -> Self
    {                   
        Self {
            iter: TaskQueue { iter },
            f:Box::pin(f),
            num_threads: Self::max_threads(),
            v: PhantomData,            
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
        
    pub fn run(self)    
    {                
        let num_threads = self.num_threads;        

        WorkerThreads { nthreads: num_threads }
        .run(self)      
        
    }
}

