//! Implementations capture the implementation of ParallelIter and IntoParallelIter
//! for commonly used collections like Vector, HashMap, Range and other relevant types

use std::collections::HashMap;
use crate::iterators::fetchdirect::{FetchDirect, FetchInDirect};

use super::{
    iterator::*,
    queued::*
};

/// Implementation for all Vectors
impl<'data, T> ParallelIter<'data,FetchInDirect<'data, T>, T> for Vec<T>
where Self: 'data
{
    type RefItem = &'data T;        
    fn parallel_iter(&'data self) -> ParallelIterator<FetchInDirect<'data,T>, Self::RefItem>   
    {       
        ParallelIterator::new(FetchInDirect::new(self))          
     }       
}

/// Implementation for all Vectors
impl<'data, T> IntoParallelIter<'data, FetchDirect<T>, T> for Vec<T>
where Self: 'data
{
    type IntoItem = T;       
    
    fn into_parallel_iter(self) -> ParallelIterator<FetchDirect<T>, Self::IntoItem> {             
        let iter =  FetchDirect::new(self);       
        ParallelIterator::new(iter)                  
    }       
}

/// Implementation for all HashMap
impl<'data, K,V> ParallelIter<'data, SizedQueue<(&'data K, &'data V)>,(&'data K, &'data V)> for HashMap<K,V>
where Self: 'data
{
    type RefItem = (&'data K, &'data V);       
    fn parallel_iter(&'data self) -> ParallelIterator<SizedQueue<(&'data K, &'data V)>, Self::RefItem>   
    {       
        let size = usize::max(self.len() / 100usize,100); 
        let input = self.iter().collect::<Vec<_>>();  
        let len = self.len();
        ParallelIterator::new(SizedQueue::new_with_size(input, size,Some(len)))  
     }          
}

impl<'data, K,V> IntoParallelIter<'data,SizedQueue<(K, V)>,(K,V)> for HashMap<K,V>
where Self: 'data
{
    type IntoItem = (K,V);
    
    fn into_parallel_iter(self) -> ParallelIterator<SizedQueue<(K, V)>, Self::IntoItem> {
        let size = usize::max(self.len() / 100usize,100); 
        let len = self.len();
        let input = self.into_iter().collect::<Vec<_>>();         
        ParallelIterator::new(SizedQueue::new_with_size(input, size,Some(len))) 
    }       
}

// Implementation for Range - usize isize i32 i64 u32 u64
macro_rules! range_impl {
    {$($T:ty)*} => {
        $(
            impl<'data> IntoParallelIter<'data,SizedQueue<$T>,$T> for std::ops::Range<$T>
            where Self: 'data
            {
                type IntoItem = $T;                
                
                fn into_parallel_iter(self) -> ParallelIterator<SizedQueue<$T>, Self::IntoItem> {                    
                    let len = self.end - self.start;  
                    let size = usize::max(len as usize / 100usize,100);                   
                    let input = self.collect::<Vec<_>>();                      
                    let x = ParallelIterator::new(SizedQueue::new_with_size(input, size,Some(len as usize)));                    
                    x
                }       
            }
        )*
    };
}

range_impl! { usize isize i32 i64 u32 u64 }
