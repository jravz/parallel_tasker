//! AtomicIterator is a trait implementd on ParallelIterator and IntoParallelIterator which are both
//! available for types implementing the Fetch trait.
//! AtomicIterator and Fetch traits together help in establishing a 1 to 1 relationship with a value
//! stored in the Collection to a usize value. Further using a counter of AtomicUsize type which is indivisible
//! it ensures that each thread does not get the same value as another thread. Allowing threads to
//! access values in the Collection in a mutually exclusive manner.

use std::{collections::HashMap, sync::{atomic::AtomicPtr, Arc}};

use crate::iterators::queued::AtomicQueuedValues;

#[allow(dead_code)]
pub trait ParallelIter<'data,T>
where Self:Sized,
{    
    type RefItem;
    type RefIterator: Iterator<Item = Self::RefItem>;    
    fn parallel_iter(&'data self) -> ParallelIterator<Self::RefIterator,Self::RefItem>;    
}

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


/// Implementation for all Vectors
impl<'data, T> ParallelIter<'data, T> for Vec<T>
where Self: 'data
{
    type RefItem = &'data T;
    type RefIterator = std::slice::Iter<'data,T>;       
    fn parallel_iter(&'data self) -> ParallelIterator<Self::RefIterator, Self::RefItem>   
    {       
        let input = self.iter();  
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000)
        }
     }       
}

/// Implementation for all Vectors
impl<'data, T> IntoParallelIter<'data, T> for Vec<T>
where Self: 'data
{
    type IntoItem = T;
    type IntoIterator = std::vec::IntoIter<T>;      
    
    fn into_parallel_iter(self) -> ParallelIterator<Self::IntoIterator, Self::IntoItem> {
        let input = self.into_iter();  
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000)
        }
    }       
}

/// Implementation for all HashMap
impl<'data, K,V> ParallelIter<'data, (K,V)> for HashMap<K,V>
where Self: 'data
{
    type RefItem = (&'data K, &'data V);
    type RefIterator = std::collections::hash_map::Iter<'data,K, V>;       
    fn parallel_iter(&'data self) -> ParallelIterator<Self::RefIterator, Self::RefItem>   
    {       
        let input = self.iter();  
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000)
        }
     }          
}

impl<'data, K,V> IntoParallelIter<'data, (K,V)> for HashMap<K,V>
where Self: 'data
{
    type IntoItem = (K,V);
    type IntoIterator = std::collections::hash_map::IntoIter<K,V>;  
    
    fn into_parallel_iter(self) -> ParallelIterator<Self::IntoIterator, Self::IntoItem> {
        let input = self.into_iter();  
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000)
        }
    }       
}

/// AtomicIterator depends on the ability to create a 1 to 1 association with a usize value less than len and a stored
/// value within the type.
/// For instance in vec![1,2,3] a usize value of 1 would give 2. 
/// In HashMap {(1,"A"), (2,"B"), (3,"B") } where collection of keys are [1,2,3], the usize 1 will be mapped to (2,"B") based on its
/// position in the collection of keys.
/// AtomicUsize and fetch_add function is used to ensure each thread gets an independent usize value that it may use to 
/// fetch a unique value from the target pool.
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
}

impl<I,T> AtomicIterator for ParallelIterator<I,T> 
where I: Iterator<Item = T>
{    
    type AtomicItem = T;
    fn atomic_next(&mut self) -> Option<T> {
        let val = self.iter.pop();        
        val
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