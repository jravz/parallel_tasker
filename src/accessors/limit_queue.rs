use crate::utils::SpinBackoff;

use super::read_accessor::*;

use std::sync::{atomic::AtomicBool, Arc};

pub struct LimitAccessQueue<T> {
    pub val: Vec<T>,    
    write_block: AtomicBool
}

#[allow(dead_code,clippy::new_ret_no_self)]
impl<T> LimitAccessQueue<T> 
{
    pub fn new() -> (PrimaryAccessor<T>,SecondaryAccessor<T>) {
        let arc_obj = Arc::new(Self {
            val: Vec::new(),            
            write_block: AtomicBool::new(false)
        });        
        
        //we need to ensure the object within AtomicPtr survives on the heap and beyond
        //the function stack. 
        // let obj_ptr = Box::into_raw(obj);
        // let arc_obj: Arc<LimitAccessQueue<T>> = Arc::new(obj);            
        let primary = ReadAccessor::new(arc_obj.clone(),ReadAccessorType::Primary);
        let secondary = ReadAccessor::new(arc_obj,ReadAccessorType::Secondary);             
        (PrimaryAccessor::new(primary), SecondaryAccessor::new(secondary))
                     
       
    }

    pub fn pop(&mut self) -> Option<T> {
        self.with_write_block(|s| { 
            s.val.pop()
        })       
    }

    pub fn steal(&mut self) -> Option<Vec<T>> {         
        self.with_write_block(|s| {
            if s.val.is_empty() {
                None
            } else {
                // using mem swap to expedite the process
                let mut tmp:Vec<T> = Vec::with_capacity(1);
                std::mem::swap(&mut tmp, &mut s.val);
                Some(tmp)            
            }        
        })              
    }

    pub fn steal_half(&mut self) -> Option<Vec<T>> {          
        self.with_write_block(|s| {         
            if s.val.is_empty() {
                None
            } 
            else {
                let halflen = s.val.len() / 2;
                let res = s.val.split_off(halflen);
                Some(res)            
            }            
        })                      
    }

    pub fn is_empty(&mut self) -> bool { 
        self.with_write_block(|s| {
            s.val.is_empty()
        })               
    }

    pub fn len(&mut self) -> usize {         
        self.with_write_block(|s|{
            if s.val.is_empty() { 0usize } else { s.val.len() }
        })        
    }

    pub fn atomic_write_block_to_true(&mut self) -> Result<bool, bool> {
        self.write_block.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
    }    

    pub fn with_write_block<F,Output>(&mut self, f:F) -> Output
    where F: FnOnce(&mut Self) -> Output {                          
        SpinBackoff::loop_while_mut(||self.atomic_write_block_to_true().is_err());                            
        let output = f(self);
        self.write_block.store(false, std::sync::atomic::Ordering::SeqCst);                 
        output
    }

    pub fn push(&mut self, value:T) {

        self.with_write_block(|s|
        {
            s.val.push(value);
        });        
    }

    pub fn write(&mut self, mut values:Vec<T>) {     
        self.with_write_block(|s|{
            let drained = values.drain(0..);                
            s.val.extend(drained); 
        });                                         
    }

    pub fn replace(&mut self, mut values:Vec<T>) {    
        self.with_write_block(|s|{             
            std::mem::swap(&mut values, &mut s.val);                      
        });                            
    }

    pub fn is_write_blocked(&self) -> bool {
        self.write_block.load(std::sync::atomic::Ordering::SeqCst)
    }

    // pub fn ingest_iter<I>(&mut self, mut i:I)
    // where I:AccessQueueIngestor<IngestorItem = T>
    // {
    //     while let Some(value) = i.next_chunk() {
    //         self.push(value);
    //     }
    // }

}