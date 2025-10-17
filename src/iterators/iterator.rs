//! AtomicIterator is a trait implementd on ParallelIterator that is then used in Map, ForEach and other similar functions which further
//! called the WorkerController to schedule and optimally run the tasks across threads.
//! SizedQueue to manage exclusive access to values within HashMap and Range. Vectors are managed via FetchDirect and FetchIndirect.
//! This is to allow faster access of Vectors as they are sequentially accessible values unlike HashMap for instance.

use std::marker::PhantomData;

/// ParallelIter gives a version of ParallelIterator that is expected to capture the .iter output
/// for those that implement the same like Vec, HashMap and so on. 
#[allow(dead_code)]
pub trait ParallelIter<'data,DiscQ, T>
where Self:Sized,
DiscQ: DiscreteQueue<Output=Self::RefItem>
{    
    type RefItem; 
    fn parallel_iter(&'data self) -> ParallelIterator<DiscQ, Self::RefItem>;    
}

/// IntoParallelIter gives a version of ParallelIterator that is expected to capture the .into_iter output
/// for those that implement the same like Vec, HashMap and so on. 
#[allow(dead_code)]
pub trait IntoParallelIter<'data,DiscQ,T>
where Self:Sized,
DiscQ: DiscreteQueue<Output=Self::IntoItem>
{    
    type IntoItem;          
    fn into_parallel_iter(self) -> ParallelIterator<DiscQ,Self::IntoItem>; 
}

#[allow(clippy::len_without_is_empty)]
pub trait DiscreteQueue 
{
    type Output;
    fn pop(&mut self) -> Option<Self::Output>;
    fn pull(&mut self) -> Option<Vec<Self::Output>>;
    fn is_active(&self) -> bool;
    fn len(&self) -> Option<usize>;
}

/// ParallelIterator is comparable to Iter, but is set up for the AtomicIterator.
pub struct ParallelIterator<DiscQ,T> 
where DiscQ: DiscreteQueue<Output=T>,
{
    pub iter:DiscQ,    
    t: PhantomData<T>
}

impl<DiscQ,T> ParallelIterator<DiscQ,T> 
where DiscQ: DiscreteQueue<Output=T>,
{
    pub fn new(iter:DiscQ) -> Self {
        Self {
            iter,            
            t:PhantomData
        }
    }
}


/// AtomicIterator trait is a special kind of iterator trait suited to enable both next and other
/// larger data pulls as demanded by the parallelism algorithm and logic 
#[allow(clippy::len_without_is_empty)]
pub trait AtomicIterator {
    type AtomicItem;
    fn atomic_next(&mut self) -> Option<Self::AtomicItem>;
    fn atomic_pull(&mut self) -> Option<Vec<Self::AtomicItem>>;
    fn len(&self) -> Option<usize>;
    ///tests whether the iterator is still active with values still available
    /// to be pulled
    fn is_active(&self) -> bool;
}

impl<DiscQ,T> AtomicIterator for ParallelIterator<DiscQ,T> 
where DiscQ:DiscreteQueue<Output = T>,
{    
    type AtomicItem = DiscQ::Output;
    fn atomic_next(&mut self) -> Option<Self::AtomicItem> {
        self.iter.pop()               
    }

    fn len(&self) -> Option<usize> {
        self.iter.len()
    }
    

    fn is_active(&self) -> bool {
        self.iter.is_active()
    }
    
    fn atomic_pull(&mut self) -> Option<Vec<Self::AtomicItem>> {
        self.iter.pull()
    }
}