//! Implementations capture the implementation of ParallelIter and IntoParallelIter
//! for commonly used collections like Vector, HashMap, Range and other relevant types

use std::collections::HashMap;
use super::{
    iterator::*,
    queued::*
};

/// Implementation for all Vectors
impl<'data, T> ParallelIter<'data, T> for Vec<T>
where Self: 'data
{
    type RefItem = &'data T;
    type RefIterator = std::slice::Iter<'data,T>;       
    fn parallel_iter(&'data self) -> ParallelIterator<Self::RefIterator, Self::RefItem>   
    {       
        let input = self.iter(); 
        let size = usize::max(self.len() / 1000usize,100);  
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, size,Some(self.len()))
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
        let size = usize::max(self.len() / 1000usize,100);  
        let len = self.len();
        let input = self.into_iter();         
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, size, Some(len))
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
        let len = self.len();
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000, Some(len))
        }
     }          
}

impl<'data, K,V> IntoParallelIter<'data, (K,V)> for HashMap<K,V>
where Self: 'data
{
    type IntoItem = (K,V);
    type IntoIterator = std::collections::hash_map::IntoIter<K,V>;  
    
    fn into_parallel_iter(self) -> ParallelIterator<Self::IntoIterator, Self::IntoItem> {
        let len = self.len();
        let input = self.into_iter();          
        ParallelIterator {
            iter: AtomicQueuedValues::new_with_size(input, 1000, Some(len))
        }
    }       
}

// Implementation for Range - usize isize i32 i64 u32 u64
macro_rules! range_impl {
    {$($T:ty)*} => {
        $(
            impl<'data> IntoParallelIter<'data,$T> for std::ops::Range<$T>
            where Self: 'data
            {
                type IntoItem = $T;
                type IntoIterator = std::ops::Range<$T>;  
                
                fn into_parallel_iter(self) -> ParallelIterator<Self::IntoIterator, Self::IntoItem> {
                    let len = self.end - self.start;                    
                    let input = self.into_iter();  
                    ParallelIterator {
                        iter: AtomicQueuedValues::new_with_size(input, 1000, Some(len as usize))
                    }
                }       
            }
        )*
    };
}

range_impl! { usize isize i32 i64 u32 u64 }
