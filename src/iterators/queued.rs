use crate::iterators::prelude::DiscreteQueue;

const QUEUE_SIZE:usize = crate::push_workers::worker_controller::INITIAL_WORKERS;

#[allow(dead_code)]
pub struct SizedQueue<I,T> 
where I: Iterator<Item = T>
{   
    queue: I,
    pull_size:usize,
    len:usize    
}

/// SizedQueue allows atomic and parallel iterator to be built over HashMaps and Ranges.
/// As given by the name it requires a known size to be specified.
/// ```
#[allow(dead_code)]
impl<I,T> SizedQueue<I,T> 
where I: Iterator<Item = T>
{
    const DEFAULT_SIZE:usize = 10;

    pub fn new(queue:I, len:usize) -> Self {        
        let pull_size = f64::ceil(len as f64 / QUEUE_SIZE as f64) as usize;
        Self {
            queue,
            pull_size,
            len               
        }       
    }
    
}

impl<I,T> DiscreteQueue for SizedQueue<I,T> 
where I: Iterator<Item = T>
{
    type Output = T;
    fn is_active(&self) -> bool {
        // self.active_status()
        true
    }

    fn pop(&mut self) -> Option<Self::Output> {                
        self.queue.next()
    }
    
    fn pull(&mut self) -> Option<Vec<Self::Output>> {
        let mut res = Vec::new();        
        for _ in 0..self.pull_size {
            let val = self.queue.next();            
            if val.is_none() { break; }
            res.push(val.unwrap());
        }
        if res.is_empty() { None } else {
            Some(res)
        }        
    }
    
    fn len(&self) -> Option<usize> {
        Some(self.len)
    }

}