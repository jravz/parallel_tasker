use std::{cell::RefCell, mem::MaybeUninit, ops::Range, sync::atomic::{AtomicBool, AtomicUsize, Ordering}};

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
        // Convert back into Vec<T> so that elements get dropped properly
        let (ptr, len, cap) = (self.buf.as_ptr(), self.buf.len(), self.buf.capacity());
        unsafe {
            Vec::from_raw_parts(ptr as *mut T, len, cap);
            // dropped automatically here as its not moved anywhere
        }
    }
}


pub struct FetchDirect<T> {
    vec: RawVec<T>,
    ctr: AtomicUsize,
    active:AtomicBool, 
    queue_size:usize,
    len:usize   
}

impl<T> FetchDirect<T> {

    pub fn new(vec:Vec<T>) -> Self {
        let max_threads = crate::utils::max_threads();
        let len = vec.len();
        let optimal_q_size = vec.len() / max_threads / QUEUE_SPLIT;                    
        Self {
            vec:RawVec::new(vec),
            ctr: AtomicUsize::new(0),
            active: AtomicBool::new(true),
            queue_size: optimal_q_size,
            len
        }
    }

}


impl<T> DiscreteQueue for FetchDirect<T> {
    type Output = T;    

    fn pop(&mut self) -> Option<Self::Output> {
        if let Some(idx) = TASK_QUEUE.with_borrow_mut(|t| t.next()){
            if idx != -1 {
                return self.vec.take(idx as usize);
            }             
        };

        let ctr = self.ctr.fetch_add(self.queue_size,Ordering::AcqRel) as isize; 
        let mut range = ctr..isize::min(ctr + self.queue_size as isize,self.vec.len() as isize);
        let opt_idx = range.next();
        TASK_QUEUE.set(range);
        if let Some(idx) = opt_idx {
            return self.vec.take(idx as usize);
        }
        None                                                          
    }

    fn pull(&mut self) -> Option<Vec<Self::Output>> {
        let ctr = self.ctr.fetch_add(self.queue_size,Ordering::AcqRel);         
        if ctr >= self.vec.len() {
            None
        } else {
            let range: Range<usize> = ctr..usize::min(ctr + self.queue_size,self.vec.len());
            let mut res = Vec::new();
            for idx in range {
                if let Some(val) = self.vec.take(idx) {
                    res.push(val);
                } else {
                    break;
                }
            }
            if res.is_empty() { return None; }
            Some(res)            
        }        
    }

    fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    fn len(&self) -> Option<usize> {
        Some(self.len)
    }
}

pub struct FetchInDirect<'data, T> {
    vec: &'data Vec<T>,
    ctr: AtomicUsize,
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
        let ctr = self.ctr.fetch_add(self.queue_size,Ordering::AcqRel);         
        if ctr >= self.vec.len() {
            None
        } else {
            let range: Range<usize> = ctr..usize::min(ctr + self.queue_size,self.vec.len());
            let mut res = Vec::new();
            for idx in range {
                if let Some(val) = self.vec.get(idx) {
                    res.push(val);
                } else {
                    break;
                }
            }
            if res.is_empty() { return None; }
            Some(res)            
        }        
    }

    fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn len(&self) -> Option<usize> {
        Some(self.len)
    }
}