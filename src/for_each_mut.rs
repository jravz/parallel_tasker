//! ParallelForEach object is implemented for AtomicIterator and hence for types implementing
//! Fetch trait. This allows an FnMut function to be run on each value of the collection implementing
//! Fetch.

const DEFAULT_THREADS_NUM:usize = 1;
const CPU_2_THREAD_RATIO:usize = 2;

use std::marker::PhantomData;
use crate::task_queue::TaskQueue;
use crate::worker_thread::WorkerThreads;
use super::iterators::iterator::AtomicIterator;

/// ParallelForEachMutIter allows calling the .for_each_mut(f) to run a FnMut function on type implementing AtomicIterator.
/// FnMut in a parallel computation does not add any value. This has been added
/// only for completion purposes.
/// ```
///  use parallel_task::{for_each_mut::ParallelForEachMutIter, prelude::*};
/// 
///  let mut test = 0;
///  let target = 100;
///  (0..=target).collect::<Vec<i32>>().
///  parallel_iter().for_each_mut(|v| { test += v;});
///  assert_eq!(test, (target * (target + 1)/2));
/// ```
/// 
pub trait ParallelForEachMutIter<I, V,F>
where Self: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send
{
    fn for_each_mut(self,f:F) {
        ParallelForEachMut::new(self,f).run()
    }
}

impl<I,V,F> ParallelForEachMutIter<I, V,F> for I 
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send,
{}

/// ParallelForEachMut is a structure type that captures the information necessary to run a FnMut closure within the Iterator in parallel
/// Its the result of parallel_task that can be run on any Iterator implementing type.
/// ```
///  use parallel_task::{for_each_mut::ParallelForEachMutIter, prelude::*};
/// 
///  let mut test = 0;
///  let target = 100;
///  (0..=target).collect::<Vec<i32>>().
///  parallel_iter().for_each_mut(|v| { test += v;});
///  assert_eq!(test, (target * (target + 1)/2));
/// ```
/// 
pub struct ParallelForEachMut<V,F,I>
where I: AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V),
V: Send
{
    pub iter: TaskQueue<I,V>,
    pub f:F,
    pub num_threads:usize,
    pub v: PhantomData<V>,    
}

#[allow(dead_code)]
impl<I,V,F> ParallelForEachMut<V,F,I>
where I:AtomicIterator<AtomicItem = V> + Send + Sized,
F: FnMut(V) + Send,
V: Send,
{
    pub fn new(iter:I,f:F) -> Self
    {                   
        Self {
            iter: TaskQueue { iter },
            f,
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
        .run_mut(self)      
        
    }
}

