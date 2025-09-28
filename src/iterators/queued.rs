use std::{cell::Cell, collections::HashMap, sync::atomic::{AtomicBool, AtomicIsize}};
const ONE_SEC:usize = 1;
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
        let mut hash:HashMap<isize,String> = HashMap::new();
        hash.insert(-100000,String::from("Default"));
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
                let mut tm = std::time::Instant::now();
                while !self.access.load(std::sync::atomic::Ordering::Relaxed)
                {                     
                    if tm.elapsed().as_secs() > 2 {
                        println!("{:?} - loop 1: {} ||",std::thread::current().id(),CURR_INDEX.get());
                        tm = std::time::Instant::now();
                    }
                }                
            }
            self.pop()                               
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
        // ensure no one is still due to pick                  
        while (self.picked_ctr.load(std::sync::atomic::Ordering::Relaxed) < self.pick_target as isize)
        && !self.queue.is_empty()
        {  
            // If this is taking more than a second then its some freak condition and no more threads are 
            // expected to be waiting for this to close.
            if tm.elapsed().as_secs() > ONE_SEC as u64 {                
                break;            
            }            
        }

        // If there are any missing they will first be picked        
        let mut new_vec = self.queue.iter_mut()
        .filter(|val| val.is_some()).map(std::mem::take).collect::<Vec<_>>();
        
        let to_fill_size = self.size - new_vec.len();
        
        // Fill the remaining values to the queue
        for _ in 0..to_fill_size {
            new_vec.push(self.iter.next());            
        }
        self.queue = new_vec;
        let to_pick = self.queue.len();
        self.ctr.store(to_pick as isize -1isize, std::sync::atomic::Ordering::Release);
        self.picked_ctr.store(0, std::sync::atomic::Ordering::SeqCst);
        self.pick_target = to_pick;
    }
}