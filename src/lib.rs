//! ParallelTaskIter is the underlying trait for a data parallelism library
//! built on the 'pull' approach.
pub mod parallel_task;
pub mod collector;
pub mod worker_thread;
pub mod errors;
pub mod prelude;
pub mod iterators;
mod iterator;

