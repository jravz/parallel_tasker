use std::sync::atomic::AtomicUsize;

#[allow(dead_code)]
pub trait ParallelIter
where Self:Sized,
{   
    type Item; 
    fn parallel_iter(&self) -> ParallelIterator<'_,Self>;
    fn atomic_get(&self, index:usize) -> Option<&Self::Item>;
    fn atomic_get_mut(&mut self, index:usize) -> Option<&mut Self::Item>;       
}


pub struct ParallelIterator<'a, T:'a> {
    item: &'a T,
    atomic_counter:AtomicUsize
}

pub struct IntoParallelIterator<T> {
    item: T,
    atomic_counter:AtomicUsize
}

impl<T> ParallelIter for Vec<T>
{    
    type Item = T;
    fn parallel_iter(&self) -> ParallelIterator<'_, Vec<T>> { 
        ParallelIterator {
            item: self,
            atomic_counter: AtomicUsize::new(0)
        }
     }   

     fn atomic_get(&self,index:usize) -> Option<&Self::Item> {
        self.get(index)
    }

    fn atomic_get_mut(&mut self,index:usize) -> Option<&mut Self::Item> {
        self.get_mut(index)
    }       
}

impl<'a,T> ParallelIterator<'a,T>
where T: ParallelIter
{
    pub fn atomic_next(&mut self) -> Option<&<T as ParallelIter>::Item> {
        let index = self.atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Acquire);
        self.item.atomic_get(index)
    }
   
}