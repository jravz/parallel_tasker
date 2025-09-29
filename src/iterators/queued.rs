use std::{cell::Cell, sync::atomic::{AtomicBool, AtomicIsize}};
#[allow(dead_code)]
const ONE_SEC:usize = 1;
const TEN:usize = 10;
#[allow(dead_code)]
pub struct AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    size: usize,    
    queue: Vec<Option<T>>,
    iter: I,
    ctr:AtomicIsize,
    access:AtomicBool,
    picked_ctr:AtomicIsize,
    pick_target:usize,
    is_active:AtomicBool,    
}

thread_local!(static CURR_INDEX: Cell<isize> = const { Cell::new(-1) });

/// AtomicQueuedValues allow parallel access across values within an iterator. It returns None when there are
/// no more values to access
/// ```
/// use parallel_task::iterators::queued::AtomicQueuedValues;
/// let mut vec = (0..1000).collect::<Vec<_>>();
/// let vec_iter = vec.into_iter();
/// let mut queue = AtomicQueuedValues::new(vec_iter);
/// assert_eq!(queue.pop(), Some(9));
/// assert_eq!(queue.pop(), Some(8));
/// ```
/// ```
/// // Testing ability to return None when there are no more elements.
/// use parallel_task::iterators::queued::AtomicQueuedValues;
/// let mut vec = vec![1];
/// let vec_iter = vec.into_iter();
/// let mut queue = AtomicQueuedValues::new(vec_iter);
/// assert_eq!(queue.pop(), Some(1));
/// assert_eq!(queue.pop(),None);
/// assert_eq!(queue.pop(),None);
/// ```
#[allow(dead_code)]
impl<T,I> AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    const DEFAULT_SIZE:usize = 10;

    pub fn new(iter:I) -> Self {
        let queue:Vec<Option<T>> = Vec::new();        
        let mut obj = Self {
            size: Self::DEFAULT_SIZE,
            queue,
            iter,
            ctr: AtomicIsize::new(-1),
            access: AtomicBool::new(true),
            picked_ctr:AtomicIsize::new(0),
            pick_target:0,
            is_active: AtomicBool::new(true)            
        };
        obj.pull_in();
        obj
    }

    pub fn new_with_size(iter:I,size:usize) -> Self {
        let queue:Vec<Option<T>> = Vec::new();      
        let mut obj = Self {
            size,
            queue,
            iter,
            ctr: AtomicIsize::new(-1),
            access: AtomicBool::new(true),
            picked_ctr:AtomicIsize::new(0),
            pick_target:0,
            is_active: AtomicBool::new(true)            
        };
        obj.pull_in();        
        obj
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Pop function retrieves a value within Some if there are values still to be retrieved.
    /// Else it returns a None
    /// ```
    /// use parallel_task::iterators::queued::AtomicQueuedValues;
    /// let mut vec = (0..1000).collect::<Vec<_>>();
    /// let vec_iter = vec.into_iter();
    /// let mut queue = AtomicQueuedValues::new(vec_iter);
    /// assert_eq!(queue.pop(), Some(9));
    /// assert_eq!(queue.pop(), Some(8));
    /// ```
    pub fn pop(&mut self) -> Option<T> {       

        let index = self.ctr.fetch_sub(1isize, std::sync::atomic::Ordering::Acquire);
        CURR_INDEX.set(index);     

        if CURR_INDEX.get() < 0 {            
            if let Ok(true) = self.access.compare_exchange(true, false, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed) 
            {                           
                self.pull_in();                         
                self.access.store(true, std::sync::atomic::Ordering::Release);
            } else {                
                while !self.access.load(std::sync::atomic::Ordering::Relaxed)
                {}                
            }

            if self.queue.len() > 0 { self.pop() }
            else { None }

        } else {                     
            let val:Option<T> = std::mem::take(&mut self.queue[CURR_INDEX.get() as usize]);                
            self.picked_ctr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);                    
            val
        }
        

    }

    /// Pulls in the next set of values from the iterator in line 
    /// with the size of the queue set at the time of creation
    fn pull_in(&mut self)
    where I: Iterator<Item = T> {

        let tm = std::time::Instant::now();
        let mut diff = 0;
        // ensure no one is still due to pick                  
        while (self.picked_ctr.load(std::sync::atomic::Ordering::Relaxed) < self.pick_target as isize)
        && !self.queue.is_empty()
        {  
            // If this is taking more than a 100ms then its some freak condition and no more threads are 
            // expected to be waiting for this to close.
            diff = self.pick_target - self.picked_ctr.load(std::sync::atomic::Ordering::Relaxed) as usize;
            // Assume 10 microseconds max per value to be picked. Counter has been selected. Only value
            // needs to be picked. Any more time could indicate some other failure
            if tm.elapsed().as_micros() as usize > TEN * diff {                
                break;            
            }            
        }        
                
        // If there are any missing they will first be picked        
        let mut new_vec = if diff > 0 {
            self.queue.iter_mut()
            .filter(|val| val.is_some()).map(std::mem::take).collect::<Vec<_>>()
        } else {
            Vec::with_capacity(self.size)
        };        

        let to_fill_size = self.size - new_vec.len();
        
        // Fill the remaining values to the queue
        for _ in 0..to_fill_size {
            let val = self.iter.next();
            if val.is_some() {
                new_vec.push(val);   
            } else {
                break;
            }
                     
        }        
        
        self.queue = new_vec;
        let to_pick = self.queue.len();        
        self.ctr.store(to_pick as isize -1isize, std::sync::atomic::Ordering::Release);
        self.picked_ctr.store(0, std::sync::atomic::Ordering::SeqCst);
        self.pick_target = to_pick;
    }
    
}