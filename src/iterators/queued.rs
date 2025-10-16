use std::{cell::Cell, sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize}};

use crate::iterators::prelude::DiscreteQueue;

const QUEUE_SIZE:usize = crate::push_workers::worker_controller::INITIAL_WORKERS;

#[allow(dead_code)]
pub struct SizedQueue<T> 
{   
    queue: Vec<T>,
    pull_size:usize     
}

// thread_local!(static CURR_INDEX: Cell<isize> = const { Cell::new(-1) });
// thread_local!(static CURR_CHECKVAL: Cell<usize> = const { Cell::new(0) });

/// SizedQueue allow parallel access across values within an iterator. It returns None when there are
/// no more values to access
/// ```
/// use parallel_task::iterators::{queued::SizedQueue,iterator::DiscreteQueue};
/// let mut rng = (0..1000);
/// let len = rng.end - rng.start;
/// let rng_iter = rng.into_iter();
/// let mut queue = SizedQueue::new_with_size(rng_iter,100, Some(len));
/// let val = queue.pop();
/// assert!(val.is_some());
/// ```
/// ```
/// // Testing ability to return None when there are no more elements.
/// use parallel_task::iterators::{queued::SizedQueue,iterator::DiscreteQueue};
/// let mut rng = (0..2);
/// let len = rng.end - rng.start;
/// let rng_iter = rng.into_iter();
/// let mut queue = SizedQueue::new_with_size(rng_iter,100, Some(len));
/// let val = queue.pop();
/// assert!(val.is_some());
/// let val = queue.pop();
/// assert!(val.is_some());
/// let val = queue.pop();
/// assert!(val.is_none());
/// ```
#[allow(dead_code)]
impl<T> SizedQueue<T> 
{
    const DEFAULT_SIZE:usize = 10;

    pub fn new(queue:Vec<T>, len:Option<usize>) -> Self {
        // let iter = iter.enumerate();
        // let queue:Vec<Option<(usize,T)>> = Vec::new();   
        // let secondary:Vec<Option<(usize,T)>> = Vec::new();
        // let test_queue = if let Some(len) = len {
        //     vec![None;len] 
        // } else {
        //     vec![None; 1000]
        // };        
        let pull_size = f64::ceil((queue.len() as f64 / QUEUE_SIZE as f64) as f64) as usize;
        Self {
            queue,
            pull_size               
        }       
    }

    pub fn new_with_size(queue:Vec<T>,size:usize, len:Option<usize>) -> Self {
        // let iter = iter.enumerate();
        // let queue:Vec<Option<(usize,T)>> = Vec::new();   
        // let secondary:Vec<Option<(usize,T)>> = Vec::new();  
        // let test_queue = if let Some(len) = len {
        //     vec![None;len] 
        // } else {
        //     vec![None; 1000]
        // };
        let pull_size = f64::ceil((queue.len() as f64 / QUEUE_SIZE as f64) as f64) as usize;
       Self {
            queue,
            pull_size               
        }                    
    }
}

impl<T> DiscreteQueue for SizedQueue<T> 
{
    type Output = T;
    fn is_active(&self) -> bool {
        // self.active_status()
        true
    }

    /// Get function retrieves a value within Some if there are values still to be retrieved.
    /// Else it returns a None
    /// ```
    /// use parallel_task::iterators::{queued::SizedQueue,iterator::DiscreteQueue};
    /// let mut rng = (0..1000);
    /// let len = rng.end - rng.start;
    /// let rng_iter = rng.into_iter();
    /// let mut queue = SizedQueue::new_with_size(rng_iter,100, Some(len));
    /// let val = queue.pop();
    /// assert!(val.is_some());
    /// ```
    fn pop(&mut self) -> Option<Self::Output> {        
        self.queue.pop()
    }
    
    fn pull(&mut self) -> Option<Vec<Self::Output>> {
        let mut res = Vec::new();        
        for _ in 0..self.pull_size {
            let val = self.pop();
            if val.is_none() { break; }
            res.push(val.unwrap());
        }
        if res.is_empty() { None } else {
            Some(res)
        }        
    }
    
    fn len(&self) -> Option<usize> {
        Some(self.queue.len())
    }

}