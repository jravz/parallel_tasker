use std::{cell::RefCell, mem::MaybeUninit, ops::Range, ptr, sync::atomic::{AtomicBool, AtomicUsize, Ordering}};

use crate::iterators::prelude::DiscreteQueue;

const QUEUE_SPLIT:usize = 2;

thread_local!(static TASK_QUEUE: RefCell<Range<isize>> = const { RefCell::new(-1..0) });

#[allow(dead_code)]
pub struct RawVec<T> 
{
    buf:Vec<MaybeUninit<T>>,
}

impl<T> RawVec<T> {
    pub fn new(vec:Vec<T>) -> Self {
        let (ptr, len, cap) = (vec.as_ptr(), vec.len(), vec.capacity());
        std::mem::forget(vec);
        let buf: Vec<MaybeUninit<T>> = unsafe { Vec::from_raw_parts(ptr as *mut MaybeUninit<T>, len, cap) };
        Self { buf}
    }

    pub fn take(&mut self, i: usize) -> Option<T> {
        if i >= self.len() {
            return None;
        }
        unsafe {
            // read T out
            let value = self.buf[i].as_ptr().read();
            // replace slot with uninit
            self.buf[i] = MaybeUninit::uninit();
            Some(value)
        }
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        // take the Vec<MaybeUninit<T>> out of self, replacing with an empty Vec
        let mut buf = std::mem::take(&mut self.buf); // self.buf becomes Vec::new()

        // Now we own the old Vec in `buf`. Get its raw parts:
        let ptr = buf.as_mut_ptr() as *mut T;
        let len = buf.len();
        let cap = buf.capacity();

        // Prevent `buf` from being dropped (which would free the allocation).
        // We're going to reconstruct a Vec<T> that will drop the allocation properly.
        std::mem::forget(buf);

        unsafe {
            // Recreate a Vec<T> with the same allocation and length so elements get dropped.
            // This is safe only if the first `len` slots are actually initialized T values.
            let _ = Vec::from_raw_parts(ptr, len, cap);
            // `_` is dropped here, running element Drop impls and deallocating once.
        }
    }
}


pub struct FetchDirect<T> {
    vec: Vec<T>,   
    queue_size:usize,
    len:usize   
}

impl<T> FetchDirect<T> {

    pub fn new(vec:Vec<T>) -> Self {
        let max_threads = crate::utils::max_threads();
        let len = vec.len();
        let optimal_q_size = vec.len() / max_threads / QUEUE_SPLIT;                    
        Self {
            vec:vec,
            queue_size: optimal_q_size,
            len
        }
    }

}


impl<T> DiscreteQueue for FetchDirect<T> {
    type Output = T;    

    fn pop(&mut self) -> Option<Self::Output> {
        self.vec.pop()                                                         
    }

    fn pull(&mut self) -> Option<Vec<Self::Output>> {        
        let size = usize::min(self.vec.len(), self.queue_size);                
        
        if size == 0 {
            None
        } else {
            Some(self.vec.drain(0..size).collect::<Vec<Self::Output>>())                 
        }         
    }

    fn is_active(&self) -> bool {
        true
    }

    fn len(&self) -> Option<usize> {
        Some(self.len)
    }
}

pub struct FetchInDirect<'data, T> {
    vec: &'data Vec<T>,
    ctr: AtomicUsize,
    start:usize,
    active:AtomicBool,
    queue_size:usize,
    len:usize       
}

impl<'data, T> FetchInDirect<'data, T> {
    pub fn new(vec: &'data Vec<T>) -> Self {
        let max_threads = crate::utils::max_threads();
        let len = vec.len();
        let optimal_q_size = vec.len() / max_threads / QUEUE_SPLIT; 
        Self {
            vec,
            start:0,
            ctr: AtomicUsize::new(0),
            active:AtomicBool::new(true),
            queue_size: optimal_q_size,
            len
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

impl<'data, T> DiscreteQueue for FetchInDirect<'data, T> {
    type Output = &'data T;

    fn pop(&mut self) -> Option<Self::Output> {
        if let Some(idx) = TASK_QUEUE.with_borrow_mut(|t| t.next()){
            if idx != -1 {                
                return self.vec.get(idx as usize);
            }             
        };

        let ctr = self.ctr.fetch_add(self.queue_size,Ordering::AcqRel) as isize; 
        let mut range = ctr..isize::min(ctr + self.queue_size as isize,self.vec.len() as isize);
        let opt_idx = range.next();
        TASK_QUEUE.set(range);
        if let Some(idx) = opt_idx {            
            return self.vec.get(idx as usize);                        
        }
        None                                                          
    }

    fn pull(&mut self) -> Option<Vec<Self::Output>> {
        let size = usize::min(self.vec.len()-self.start,self.queue_size);
        if size == 0 {
            None
        } else {

            let start = self.start;
            let end = start+ size;
            self.start += size;
            Some(self.vec[start..end].iter().collect::<Vec<Self::Output>>())                
        }       
    }

    fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn len(&self) -> Option<usize> {
        Some(self.len)
    }
}