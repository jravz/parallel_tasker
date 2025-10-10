//! A super fast data parallelism library using Atomics to share data across threads and uniquely pull values 
//! from Collections such as Vec or HashMap. It follows a 'pull' approach and tries to reduce the time required 
//! for the thread to pick the next value. The key question was can the tine taken by thread to get the next job 
//! be kept minimal. This is achieved by using an AtomicIterator that generates a mutually exclusive usize value for each 
//! thread that corresponds to a unique stored value in the Collection. 
//! Hence, all types that implement the Fetch Trait (Vec and HashMap) can be consumed or passed by reference to run 
//! Map or ForEach functions on the same.

//! The results show comparable performance to the popular Rayon library and in a number of cases improved performance as well. 
//! No study has been done to establish whether the results are significant.
//! 
//! 
//! 
//! Add this crate using:
//! 
//! cargo add parallel_task
//! 
//! 
//! Code sample below:
//! 
//! ```
//! use parallel_task::prelude::*;
//! let job = || {              
//! 
//!     std::thread::sleep(std::time::Duration::from_nanos(10)); 
//!     (0..1_000).sum::<i32>()
//!  };
//! let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>(); 
//! // Parallel Iter example
//! let r1 = vec_jobs.parallel_iter().map(|func| func()).collect::<Vec<i32>>();
//! // Into Parallel Iter that consumes the vec_jobs
//! let r1 = vec_jobs.into_parallel_iter().map(|func| func()).collect::<Vec<i32>>();
//! // Print all values using a for_each. This runs for_each concurrently on a Vec or HashMap
//! r1.parallel_iter().for_each(|val| { print!("{} ",*val);});
//! ```
//! 
pub mod map;
pub mod collector;
pub mod worker_thread;
pub mod errors;
pub mod prelude;
pub mod iterators;
pub mod task_queue;
pub mod for_each;
pub mod utils;
pub mod push_workers;