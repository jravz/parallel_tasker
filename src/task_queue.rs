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
    pub fn pop(&mut self) -> Option<V> {
        self.iter.atomic_next()
    }
}