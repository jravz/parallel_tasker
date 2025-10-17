//! Structure to allow direct and by reference fetching of values from Vectors.

use crate::iterators::prelude::DiscreteQueue;

// Initial workers are part of task scheduling algorithm used to decide number of initial threads that are launched.
const QUEUE_SPLIT:usize = crate::push_workers::worker_controller::INITIAL_WORKERS;

pub struct FetchDirect<T> {
    vec: Vec<T>,   
    queue_size:usize,    
}

impl<T> FetchDirect<T> {

    pub fn new(vec:Vec<T>) -> Self {                
        let optimal_q_size = vec.len() / QUEUE_SPLIT;                    
        Self {
            vec,
            queue_size: optimal_q_size,            
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
        !self.vec.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.vec.len())
    }
}

pub struct FetchInDirect<'data, T> {
    vec: &'data Vec<T>,    
    start:usize,    
    queue_size:usize,    
}

impl<'data, T> FetchInDirect<'data, T> {
    pub fn new(vec: &'data Vec<T>) -> Self {        
        let optimal_q_size = vec.len() / QUEUE_SPLIT; 
        Self {
            vec,
            start:0,                        
            queue_size: optimal_q_size,            
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

impl<'data, T> DiscreteQueue for FetchInDirect<'data, T> {
    type Output = &'data T;

    fn pop(&mut self) -> Option<Self::Output> {
        let start = self.start;
        self.start += 1;
        self.vec.get(start)                                
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
       !self.vec.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}