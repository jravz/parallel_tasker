//! Adding this module as parallel_task::prelude::* gives access to the desired
//! functionalities.
//! 

pub use crate::iterators::prelude::{AtomicIterator,ParallelIter,IntoParallelIter};
pub use crate::{
    map::ParallelMapIter,
    for_each::ParallelForEachIter,
    for_each_mut::ParallelForEachMutIter
};
pub use crate::task_queue::TaskQueue;