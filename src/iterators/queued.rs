use std::iter::Take;

pub(crate) struct QueuedValues<I,T> 
where I: Iterator<Item = T>
{
    size: usize,    
    queue: Vec<T>,
    iter: I,
    ctr:isize
}

impl<T,I> QueuedValues<I,T> 
where I: Iterator<Item = T>
{
    pub fn new(iter:I, size:usize) -> Self {
        let queue:Vec<T> = Vec::new();
        Self {
            size,
            queue,
            iter,
            ctr: -1isize
        }
    }

    pub fn pop(&mut self,index:usize) -> Option<T> {
        if self.ctr == -1 {
            self.pull_in();
        }

        let pos = index % self.size;

    }

    pub fn pull_in(&mut self)
    where I: Iterator<Item = T> {
        let mut new_vec = Vec::new();
        for _ in 0..self.size {
            if let Some(val) = self.iter.next() {
                new_vec.push(val);
            } else {
                break;
            };
        }
        self.queue = new_vec;
    }
}