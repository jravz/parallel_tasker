use std::sync::atomic::AtomicUsize;

use crate::iterators::fetch::Fetch;

#[allow(dead_code)]
pub trait ParallelIter:Fetch
where Self:Sized,
{       
    fn parallel_iter(&self) -> ParallelIterator<'_,Self>;
    fn into_parallel_iter(self) -> IntoParallelIterator<Self>; 
}

/// ParallelIterator is comparable to Iter, but is set up for the AtomicIterator.
/// Collection keys are only stored in the case of types like HashMap where the keys collection
/// is used to establish a unique 1 to 1 relationship with a usize value from atomic_counter.
pub struct ParallelIterator<'a, T:Fetch + 'a> 
{
    item: &'a T,
    atomic_counter:AtomicUsize,
    collection_keys:Vec<<T as Fetch>::FetchKey>
}

/// IntoParallelIterator is comparable to IntoIter, but is set up for the AtomicIterator.
/// Collection keys are only stored in the case of types like HashMap where the keys collection
/// is used to establish a unique 1 to 1 relationship with a usize value from atomic_counter.
pub struct IntoParallelIterator<T:Fetch> {
    item: T,
    atomic_counter:AtomicUsize,    
    collection_keys:Vec<<T as Fetch>::FetchKey>
}

/// Blanket implementation for all T that implement Fetch
impl<T:Fetch> ParallelIter for T
{        
    fn parallel_iter(&self) -> ParallelIterator<'_, T> {         
        ParallelIterator {
            item: self,
            atomic_counter: AtomicUsize::new(0),
            collection_keys: <T as Fetch>::keys_vec(self)
        }
     }      
    
    fn into_parallel_iter(self) -> IntoParallelIterator<Self> {
        let collection_keys:Vec<<T as Fetch>::FetchKey> = <T as Fetch>::keys_vec(&self);
        IntoParallelIterator {
            item: self,
            atomic_counter: AtomicUsize::new(0),
            collection_keys
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
}

impl<'a,T:Fetch> AtomicIterator for ParallelIterator<'a,T> {
    type AtomicItem = <T as Fetch>::FetchRefItem<'a>;
    fn atomic_next(&mut self) -> Option<Self::AtomicItem> {
        let index = self.atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Acquire);
        let key = <T as Fetch>::get_key(&self.collection_keys, &index);
        if let Some(key) = key {
            self.item.atomic_get(key)
        } else {
            None
        }
    }
}

impl<T:Fetch> AtomicIterator for IntoParallelIterator<T> {
    type AtomicItem = <T as Fetch>::FetchedItem;
    fn atomic_next(&mut self) -> Option<Self::AtomicItem> {
        let index = self.atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Acquire);
        let key = <T as Fetch>::get_key(&self.collection_keys, &index);
        if let Some(key) = key {
            self.item.atomic_fetch(key)
        } else {
            None
        }
    }
}

