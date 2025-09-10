//! ParallelTaskIter is the underlying trait for a data parallelism library
//! built on the 'pull' approach.
pub mod map;
pub mod collector;
pub mod worker_thread;
pub mod errors;
pub mod prelude;
pub mod iterators;
pub mod task_queue;
pub mod for_each;