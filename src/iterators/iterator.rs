use std::sync::atomic::AtomicUsize;

use crate::iterators::fetch::Fetch;

#[allow(dead_code)]
pub trait ParallelIter:Fetch
where Self:Sized,
{       
    fn parallel_iter(&self) -> ParallelIterator<'_,Self>;
    fn into_parallel_iter(self) -> IntoParallelIterator<Self>; 
}


pub struct ParallelIterator<'a, T:Fetch + 'a> 
{
    item: &'a T,
    atomic_counter:AtomicUsize,
    collection_keys:Vec<<T as Fetch>::FetchKey>
}

pub struct IntoParallelIterator<T:Fetch> {
    item: T,
    atomic_counter:AtomicUsize,    
    collection_keys:Vec<<T as Fetch>::FetchKey>
}


impl<T:Fetch> ParallelIter for T
{        
    fn parallel_iter(&self) -> ParallelIterator<'_, T> {         
        ParallelIterator {
            item: self,
            atomic_counter: AtomicUsize::new(0),
            collection_keys: <T as Fetch>::keys_vec(&self)
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

