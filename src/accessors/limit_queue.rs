use std::{hint::spin_loop, sync::{atomic::{AtomicBool, AtomicPtr}, Arc}};

pub struct LimitAccessQueue<T> {
    val: Vec<T>,
    primary_block: AtomicBool,
    write_block: AtomicBool
}

pub enum ReadAccessorType {
    Primary,
    Secondary
}

pub struct ReadAccessor<T> 
{
    val: Arc<AtomicPtr<LimitAccessQueue<T>>> ,
    rtype: ReadAccessorType
}

#[allow(dead_code)]
impl<T> ReadAccessor<T> 
{

    fn is_blocked(&self) -> bool {
        unsafe {
            if let Some(val) = (self.val.load(std::sync::atomic::Ordering::Acquire)).as_ref() {
                val.is_blocked()
            } else {
                true
            }
        }        
    }

    fn block(&self) -> Result<bool,bool> {        
        if let Some(obj) = self.get_mut() {
            obj.block()
        } else {
            panic!("Error in accessing limit queue accessor object.");
        }
    }

    fn unblock(&self) -> Result<bool,bool> {        
        if let Some(obj) = self.get_mut() {
            obj.unblock()
        } else {
            panic!("Error in accessing limit queue accessor object.");
        }
    }

    fn get_ref(&self) -> Option<&LimitAccessQueue<T>> {
        unsafe {
             if let Some(val) = (self.val.load(std::sync::atomic::Ordering::Acquire)).as_ref() {
                Some(val)
            } else {
                None
            }
        }       
    }

    fn get_mut(&self) -> Option<&mut LimitAccessQueue<T>> {        
        unsafe {
             if let Some(val) = (self.val.load(std::sync::atomic::Ordering::Acquire)).as_mut() {
                Some(val)
            } else {
                None
            }
        }       
    } 

    pub fn pop(&self) -> Option<T> {
        unsafe {  

            while self.block().is_err() {
                spin_loop();
            }

            let res = if let Some(val) = self.val.load(std::sync::atomic::Ordering::Acquire).as_mut() {
                val.pop()
            } else {
                None
            };

            _ = self.unblock();

            res
        }                
    }

    pub fn is_empty(&self) -> bool {
        if let Some(obj) = self.get_mut() {
            while obj.block().is_err() {spin_loop();}
            let empty = obj.val.is_empty();
            while obj.unblock().is_err() {spin_loop();}
            return empty;
        }
        true
    }

    pub fn len(&self) -> usize {
        if let Some(obj) = self.get_mut() {
            while obj.block().is_err() {spin_loop();}
            let len = obj.len();
            while obj.unblock().is_err() {spin_loop();}
            return len;
        }
        0_usize
    }

    pub fn write(&mut self, values:Vec<T>) -> Result<bool,bool> {
        if let Some(obj) = self.get_mut() {
            obj.write(values);
            return Ok(true);
        } else {
            return Err(false);
        }
    }

    pub fn replace(&self, values:Vec<T>) -> Result<bool,bool> {
        if let Some(obj) = self.get_mut() {
            obj.replace(values);
            return Ok(true);
        } else {
            return Err(false);
        }
    }

    pub fn is_write_blocked(&self) -> bool {
        if let Some(obj) = self.get_ref() {
            obj.is_write_blocked()
        } else {
            true
        }
    }

    pub fn steal(&mut self) -> Option<Vec<T>> {
        match self.rtype {
            ReadAccessorType::Secondary => {
                None
            }
            ReadAccessorType::Primary => {                
                if let Some(obj) = self.get_mut() {
                    obj.steal()
                } else {
                    None
                }
            }
        }
    }

    pub fn steal_half(&mut self) -> Option<Vec<T>> {
        match self.rtype {
            ReadAccessorType::Secondary => {
                None
            }
            ReadAccessorType::Primary => {                
                if let Some(obj) = self.get_mut() {
                    obj.steal()
                } else {
                    None
                }
            }
        }
    }
}

#[allow(dead_code)]
impl<T> LimitAccessQueue<T> 
{
    pub fn new() -> (ReadAccessor<T>,ReadAccessor<T>) {
        let obj = Box::new(Self {
            val: Vec::new(),
            primary_block: AtomicBool::new(false),
            write_block: AtomicBool::new(false)
        });

        //we need to ensure the object within AtomicPtr survives on the heap and beyond
        //the function stack. 
        let obj_ptr = Box::into_raw(obj);
        let arc_obj: Arc<AtomicPtr<LimitAccessQueue<T>>> = Arc::new(AtomicPtr::new(obj_ptr));
        let primary = ReadAccessor {
            val: arc_obj.clone(),
            rtype: ReadAccessorType::Primary
        };

        let secondary = ReadAccessor {
            val: arc_obj,
            rtype: ReadAccessorType::Secondary
        };     

        (primary,secondary)
    }

    pub fn block(&mut self) -> Result<bool,bool> {
        self.primary_block.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn unblock(&mut self) -> Result<bool,bool> {
        self.primary_block.compare_exchange(true, false, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_blocked(&self) -> bool {
        self.primary_block.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.val.pop()
    }

    pub fn steal(&mut self) -> Option<Vec<T>> { 
        // first block before you steal       
        while self.block().is_err() {
            spin_loop();
        };

        let response = if self.val.is_empty() {
            None
        } else {
            Some(self.val.drain(0..).collect::<Vec<T>>())
        };
        if self.unblock().is_err() {
            panic!("Unknown error occurred when stealing from the queue");
        };
        response       
    }

    pub fn steal_half(&mut self) -> Option<Vec<T>> { 
        // first block before you steal       
        while self.block().is_err() {
            spin_loop();
        };

        let response = if self.val.is_empty() {
            None
        } else {
            let halflen = self.len() / 2;
            Some(self.val.drain(0..halflen).collect::<Vec<T>>())            
        };
        if self.unblock().is_err() {
            panic!("Unknown error occurred when stealing from the queue");
        };
        response       
    }

    pub fn len(&self) -> usize {
        self.val.len()
    }

    pub fn write(&mut self, mut values:Vec<T>) {                               
        // wait till you have permission to write and then block
        while self.write_block.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst) 
        .is_err()
        {     
            spin_loop();   
        }        
        let drained = values.drain(0..);                
        self.val.extend(drained);        
        //restore permission to write
        self.write_block.store(false, std::sync::atomic::Ordering::SeqCst);        
    }

    pub fn replace(&mut self, mut values:Vec<T>) {                       
        // wait till you have permission to write and then block
        while self.write_block.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst) 
        .is_err()
        {     
            spin_loop();   
        }
        std::mem::swap(&mut values, &mut self.val);             
        //restore permission to write
        self.write_block.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_write_blocked(&self) -> bool {
        self.write_block.load(std::sync::atomic::Ordering::SeqCst)
    }

}