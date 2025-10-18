//! A fast data parallelism library using Atomics to share data across threads and uniquely pull values 
//! from Collections such as Vec, Range or HashMap. It follows a 'push' approach and has a scheduling algorithm based
//! on work redistribution to expedite results. 
//! The results show comparable performance to the popular Rayon library within 5 - 10%. 
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
pub mod accessors;