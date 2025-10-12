use std::{cell::Cell, sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize}};

use crate::iterators::prelude::DiscreteQueue;

#[allow(dead_code)]
pub struct AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    size: usize,
    len: Option<usize>,    
    queue: Vec<Option<(usize,T)>>,
    secondary:Vec<Option<(usize,T)>>,
    iter: I,
    ctr:AtomicIsize,
    layclaim:AtomicUsize,
    access:AtomicBool,
    access_secondary:AtomicBool,
    access_primary: AtomicBool,    
    check_val:AtomicUsize,
    is_active:AtomicBool, 
    test_queue:Vec<Option<bool>>  
}

thread_local!(static CURR_INDEX: Cell<isize> = const { Cell::new(-1) });
thread_local!(static CURR_CHECKVAL: Cell<usize> = const { Cell::new(0) });

/// AtomicQueuedValues allow parallel access across values within an iterator. It returns None when there are
/// no more values to access
/// ```
/// use parallel_task::iterators::{queued::AtomicQueuedValues,iterator::DiscreteQueue};
/// let mut rng = (0..1000);
/// let len = rng.end - rng.start;
/// let rng_iter = rng.into_iter();
/// let mut queue = AtomicQueuedValues::new_with_size(rng_iter,100, Some(len));
/// let val = queue.pop();
/// assert!(val.is_some());
/// ```
/// ```
/// // Testing ability to return None when there are no more elements.
/// use parallel_task::iterators::{queued::AtomicQueuedValues,iterator::DiscreteQueue};
/// let mut rng = (0..2);
/// let len = rng.end - rng.start;
/// let rng_iter = rng.into_iter();
/// let mut queue = AtomicQueuedValues::new_with_size(rng_iter,100, Some(len));
/// let val = queue.pop();
/// assert!(val.is_some());
/// let val = queue.pop();
/// assert!(val.is_some());
/// let val = queue.pop();
/// assert!(val.is_none());
/// ```
#[allow(dead_code)]
impl<T,I> AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    const DEFAULT_SIZE:usize = 10;

    pub fn new(iter:I, len:Option<usize>) -> Self {
        // let iter = iter.enumerate();
        let queue:Vec<Option<(usize,T)>> = Vec::new();   
        let secondary:Vec<Option<(usize,T)>> = Vec::new();
        let test_queue = if let Some(len) = len {
            vec![None;len] 
        } else {
            vec![None; 1000]
        };        
        
        Self {
            size: Self::DEFAULT_SIZE,
            len,
            queue,
            secondary,
            iter,
            ctr: AtomicIsize::new(-1),
            layclaim:AtomicUsize::new(0),
            access: AtomicBool::new(true),
            access_secondary: AtomicBool::new(true),
            access_primary: AtomicBool::new(true),                     
            check_val: AtomicUsize::new(0),                     
            is_active: AtomicBool::new(true)   ,
            test_queue        
        }       
    }

    pub fn new_with_size(iter:I,size:usize, len:Option<usize>) -> Self {
        // let iter = iter.enumerate();
        let queue:Vec<Option<(usize,T)>> = Vec::new();   
        let secondary:Vec<Option<(usize,T)>> = Vec::new();  
        let test_queue = if let Some(len) = len {
            vec![None;len] 
        } else {
            vec![None; 1000]
        };
        
        Self {
            size,
            queue,
            len,
            secondary,
            iter,
            ctr: AtomicIsize::new(-1),
            layclaim:AtomicUsize::new(0),
            access: AtomicBool::new(true),
            access_secondary: AtomicBool::new(true),
            access_primary: AtomicBool::new(true),                 
            check_val: AtomicUsize::new(0),                          
            is_active: AtomicBool::new(true) ,
            test_queue           
        }                     
    }

    // pub fn active_status(&self) -> bool {
    //     self.is_active.load(std::sync::atomic::Ordering::Relaxed)
    // }

    // fn add_test_queue(&mut self, val:usize) -> bool {                
    //     let len = self.test_queue.len();
    //     if val < len {
    //         let presence = self.test_queue[val];
    //         if presence.is_some() {
    //             return false;
    //         } else {
    //             if let Some(max) = self.len {
    //                 if val >= max {
    //                     println!("({}: {})",val,max);
    //                     return false;
    //                 }
    //             }
    //             self.test_queue[val] = Some(true);
    //             return true;
    //         }
    //     }

    //     if val > self.test_queue.capacity() {
    //         self.test_queue.reserve(val + 1000);
    //     }
        
    //     for _ in len..val {
    //         self.test_queue.push(None);
    //     }
    //     self.test_queue.push(Some(true));
    //     true
    // }
    
    // fn get(&mut self) -> Option<T> {       

    //     let index = self.ctr.fetch_sub(1isize, std::sync::atomic::Ordering::Acquire);        
    //     CURR_INDEX.set(index);                    

    //     if self.secondary_needs_filling() {
    //         self.prep_secondary();
    //     }    

    //     if CURR_INDEX.get() < 0 {                     
    //         if let Ok(true) = self.access.compare_exchange(true, false, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed) {                    
    //             self.disable_primary_access();                                
    //             self.pull_in(); 
    //             self.enable_primary_access();                                     
    //             self.access.store(true, std::sync::atomic::Ordering::Release);
    //         } else {                
    //             while !self.access.load(std::sync::atomic::Ordering::Relaxed)
    //             {
    //                 std::hint::spin_loop()
    //             }                
    //         }

    //         if !self.queue.is_empty() { self.pop() }
    //         else { None }
    //     } else if let Some(val) = self.get_value() {
    //         val                  
    //     } else {
    //         self.pop()            
    //     }        
    // }


    // fn secondary_needs_filling(&self) -> bool {
    //     if !self.is_active.load(std::sync::atomic::Ordering::Relaxed) { return false; }
    //     if CURR_INDEX.get() <= 0 { return false; }
    //     if self.secondary.len() >= self.size { return false;}
    //     let is_required = self.queue.len() as f64 / CURR_INDEX.get() as f64;
    //     if is_required < 2.0 { return false; }        

    //     self.access_secondary.compare_exchange(true, false, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
    //     .is_ok()                       
    // }

    // fn prep_secondary(&mut self) {    
    //     let to_fill_size = self.size - self.secondary.len();
    //     // Fill the remaining values to the queue
    //     for _ in 0..to_fill_size {
    //         let val = self.iter.next();
    //         if val.is_some() {                                
    //             self.secondary.push(val);
    //         } else {
    //             self.is_active.store(false, std::sync::atomic::Ordering::Release);
    //             break;
    //         }                                       
    //     }         
    //     self.access_secondary.store(true, std::sync::atomic::Ordering::SeqCst);        
    // }

    // fn get_value(&mut self) -> Option<Option<T>> { 
    //     if self.access_primary.load(std::sync::atomic::Ordering::SeqCst) {                                                                                                           
    //         if let Some(val) = self.queue.get_mut(CURR_INDEX.get() as usize) {                               
    //             self.layclaim.fetch_add(1, std::sync::atomic::Ordering::Release);                                                                                                       
    //             if let Some((pos,value)) = std::mem::take(val) {                                                  
    //                 if self.add_test_queue(pos) {  
    //                     self.layclaim.fetch_sub(1, std::sync::atomic::Ordering::Release);                      
    //                     return Some(Some(value));                           
    //                 }                                                                            
    //             } 
    //             self.layclaim.fetch_sub(1, std::sync::atomic::Ordering::Release);                                                                                                                                                                                 
    //         }   
    //     }                                                                                                                                                    
    //     None
    // }

    // fn disable_primary_access(&mut self) {        
    //     while self.access_primary
    //     .compare_exchange(true, false, std::sync::atomic::Ordering::SeqCst,std::sync::atomic::Ordering::SeqCst)
    //     .is_err() {};
    // }

    // fn enable_primary_access(&mut self) {
    //     self.access_primary.store(true, std::sync::atomic::Ordering::SeqCst);
    // }


    // /// Pulls in the next set of values from the iterator in line 
    // /// with the size of the queue set at the time of creation
    // fn pull_in(&mut self)
    // where I: Iterator<Item = T> {                        
    //     let mut new_vec:Vec<Option<(usize,T)>> = Vec::with_capacity(self.size);
    //     std::mem::swap(&mut self.queue, &mut new_vec);   

    //     //Get access and block secondary prior to this activity
    //     while self.access_secondary
    //     .compare_exchange(true, false, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed)
    //     .is_err(){}                                                 
        
    //     while self.layclaim.load(std::sync::atomic::Ordering::SeqCst) > 0 {}

    //     new_vec = new_vec.into_iter().filter(|v| v.is_some())
    //     .collect::<Vec<_>>();
    //     let to_fill = usize::min(self.size - new_vec.len(), self.secondary.len());
    //     new_vec.extend(self.secondary.drain(..to_fill));                              

    //     //release secondary
    //     self.access_secondary.store(true, std::sync::atomic::Ordering::Release);   
     
    //     let to_fill_size = self.size - new_vec.len();
    //     for _ in 0..to_fill_size {            
    //         let val = self.iter.next();
    //         if val.is_some() {
    //             new_vec.push(val);
    //         } else {
    //             self.is_active.store(false, std::sync::atomic::Ordering::Release);
    //             break;
    //         }                                 
    //     }        

    //     self.queue = new_vec;
    //     let to_pick = self.queue.len();                
    //     self.ctr.store(to_pick as isize - 1isize, std::sync::atomic::Ordering::Release);             
    // }
    
}

impl<T,I> DiscreteQueue for AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    type Output = T;
    fn is_active(&self) -> bool {
        // self.active_status()
        true
    }

    /// Get function retrieves a value within Some if there are values still to be retrieved.
    /// Else it returns a None
    /// ```
    /// use parallel_task::iterators::{queued::AtomicQueuedValues,iterator::DiscreteQueue};
    /// let mut rng = (0..1000);
    /// let len = rng.end - rng.start;
    /// let rng_iter = rng.into_iter();
    /// let mut queue = AtomicQueuedValues::new_with_size(rng_iter,100, Some(len));
    /// let val = queue.pop();
    /// assert!(val.is_some());
    /// ```
    fn pop(&mut self) -> Option<Self::Output> {
        self.iter.next()
    }
    
    fn pull(&mut self) -> Option<Vec<Self::Output>> {
        let mut res = Vec::new();
        for _ in 0..500 {
            let val = self.pop();
            if val.is_none() { break; }
            res.push(val.unwrap());
        }
        if res.is_empty() { None } else {
            Some(res)
        }        
    }
    
    fn len(&self) -> Option<usize> {
        self.len
    }

}