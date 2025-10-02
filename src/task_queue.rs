//! TaskQueue stores the AtomicIterator that allows a unique value to be popped up for each
//! thread that enquires the same. 
//! 
use crate::prelude::AtomicIterator;

pub struct TaskQueue<I,V> 
where I:AtomicIterator<AtomicItem = V> + Send + Sized,
V: Send
{
    pub iter: I
}

impl<I,V> TaskQueue<I,V> 
where I:AtomicIterator<AtomicItem = V> + Send + Sized,
V:Send
{
    /// Pops a unique value from the TaskQueue. It returns None when all values are done, which is the 
    /// signal for the threads to (return and) exit.
    pub fn pop(&mut self) -> Option<V> {
        // let tm = std::time::Instant::now();
        // let x = self.iter.atomic_next();
        // println!("Pop time = {}",tm.elapsed().as_nanos());
        // x
        self.iter.atomic_next()
    }
}