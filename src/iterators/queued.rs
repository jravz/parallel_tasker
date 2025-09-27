use std::sync::atomic::{AtomicBool, AtomicIsize};

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
    pick_target:usize
}

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
/// 

#[allow(dead_code)]
impl<T,I> AtomicQueuedValues<I,T> 
where I: Iterator<Item = T>
{
    const DEFAULT_SIZE:usize = 10;

    pub fn new(iter:I) -> Self {
        let queue:Vec<Option<T>> = Vec::new();
        Self {
            size: Self::DEFAULT_SIZE,
            queue,
            iter,
            ctr: AtomicIsize::new(-1),
            access: AtomicBool::new(true),
            picked_ctr:AtomicIsize::new(0),
            pick_target:0
        }
    }

    pub fn new_with_size(iter:I,size:usize) -> Self {
        let queue:Vec<Option<T>> = Vec::new();
        Self {
            size,
            queue,
            iter,
            ctr: AtomicIsize::new(-1),
            access: AtomicBool::new(true),
            picked_ctr:AtomicIsize::new(0),
            pick_target:0
        }
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

        if index < 0 {
            if let Ok(true) = self.access.compare_exchange(true, false, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed) 
            {
                self.pull_in();
                self.access.store(true, std::sync::atomic::Ordering::Release);
            } else {
                while self.access.load(std::sync::atomic::Ordering::Relaxed) == false
                {}
            }

            if self.queue.len() < 1 {
                return None;
            } else {
                return self.pop()
            }                        
        } else {
            // println!("{:?}:getting the value:",std::thread::current().id());
            let val:Option<T> = std::mem::take(&mut self.queue[index as usize]);
            self.picked_ctr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return val;
        }
        

    }

    /// Pulls in the next set of values from the iterator in line 
    /// with the size of the queue set at the time of creation
    fn pull_in(&mut self)
    where I: Iterator<Item = T> {

        // ensure no one is still due to pick
        while self.picked_ctr.load(std::sync::atomic::Ordering::Relaxed) < self.pick_target as isize {}

        let mut new_vec = Vec::new();
        for _ in 0..self.size {
            if let Some(val) = self.iter.next() {
                new_vec.push(Some(val));
            } else {
                break;
            };
        }
        self.queue = new_vec;
        let to_pick = self.queue.len();
        self.ctr.store(to_pick as isize -1isize, std::sync::atomic::Ordering::Release);
        self.picked_ctr.store(0 as isize, std::sync::atomic::Ordering::SeqCst);
        self.pick_target = to_pick;


    }
}