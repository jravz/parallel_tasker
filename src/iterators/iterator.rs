//! AtomicIterator is a trait implementd on ParallelIterator and IntoParallelIterator that both employ 
//! AtomicQueuedValues to manage exclusive access to values within Vec, HashMap that implement Iterator.
//! AtomicQueuedValues uses Atomics to ensure that each thread does not get the same value as another thread. Allowing threads to
//! access values in the Collection in a mutually exclusive manner.

use std::sync::{atomic::AtomicPtr, Arc};

use crate::iterators::queued::AtomicQueuedValues;

/// ParallelIter gives a version of ParallelIterator that is expected to capture the .iter output
/// for those that implement the same like Vec, HashMap and so on. 
#[allow(dead_code)]
pub trait ParallelIter<'data,T>
where Self:Sized,
{    
    type RefItem;
    type RefIterator: Iterator<Item = Self::RefItem>;    
    fn parallel_iter(&'data self) -> ParallelIterator<Self::RefIterator,Self::RefItem>;    
}

/// IntoParallelIter gives a version of ParallelIterator that is expected to capture the .into_iter output
/// for those that implement the same like Vec, HashMap and so on. 
#[allow(dead_code)]
pub trait IntoParallelIter<'data,T>
where Self:Sized,
{    
    type IntoItem;    
    type IntoIterator: Iterator<Item = Self::IntoItem>;    
    fn into_parallel_iter(self) -> ParallelIterator<Self::IntoIterator,Self::IntoItem>; 
}

/// ParallelIterator is comparable to Iter, but is set up for the AtomicIterator.
pub struct ParallelIterator<I, T> 
where I: Iterator<Item = T>
{
    pub iter: AtomicQueuedValues<I,T>,    
}


/// AtomicIterator trait is applied on the  ParallelIterator that has AtomicQueuedValues 
/// due to which it is able to manage exclusive access for each thread for values within
/// the implemented Collection type. 
pub trait AtomicIterator {
    type AtomicItem;
    fn atomic_next(&mut self) -> Option<Self::AtomicItem>;

    /// create a shareable iterator for safe access across threads without
    /// any overlaps
    fn shareable(self) -> Arc<ShareableAtomicIter<Self>> 
    where Self:Sized
    {
        Arc::new(ShareableAtomicIter::new(self))
    }

    ///tests whether the iterator is still active with values still available
    /// to be pulled
    fn is_active(&self) -> bool;
}

impl<I,T> AtomicIterator for ParallelIterator<I,T> 
where I: Iterator<Item = T>
{    
    type AtomicItem = T;
    fn atomic_next(&mut self) -> Option<T> {
        self.iter.pop()               
    }

    fn is_active(&self) -> bool {
        self.iter.is_active()
    }
}

/// ShareableAtomicIter enables Vec and HashMap that implement the 
/// Fetch trait to easily distributed across threads. Values can be safely
/// accessed without any risk of overlaps. Thus allowing you to design how you 
/// wish to process these collections across your threads
/// ```
/// use parallel_task::prelude::*;
/// 
/// // Test out the AtomicIterator for parallel management of Vectors without risk of overlaps
///    let values = (0..100).collect::<Vec<_>>();
///    std::thread::scope(|s| 
///    {
///      let shared_vec = values.into_parallel_iter().shareable();
///      let share_clone = shared_vec.clone();
///      s.spawn(move || {
///         let tid = std::thread::current().id();
///         while let Some(val) = shared_vec.next(){
///             print!(" [{:?}: {}] ",tid,val);
///         }
///         });
///      s.spawn(move || {
///         let tid = std::thread::current().id();
///         while let Some(val) = share_clone.next(){
///             print!(" [{:?}: {}] ",tid,val);
///         }
///         });
///      }
/// );
/// ```
pub struct ShareableAtomicIter<T> 
where T: AtomicIterator
{
    ptr: AtomicPtr<T>
}

impl<T> ShareableAtomicIter<T> 
where T: AtomicIterator {

    pub fn new(val:T) -> Self {

        let ptr = Box::into_raw(Box::new(val));

        ShareableAtomicIter {
            ptr: AtomicPtr::new(ptr)
        }
    }

    pub fn next(&self) -> Option<<T as AtomicIterator>::AtomicItem> {
        unsafe {
            if let Some(mutable) = self.ptr.load(std::sync::atomic::Ordering::Acquire).as_mut() {
                return mutable.atomic_next();
            };
        }        
        None
    }
}