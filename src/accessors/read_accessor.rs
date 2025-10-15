use std::{hint::spin_loop, ops::{Deref, DerefMut}, sync::{atomic::AtomicPtr, Arc}};

use crate::{accessors::limit_queue::LimitAccessQueue, utils::SpinWait};


/// Add a primary and secondary accessor to easily differentiate during usage
/// and at the time of creation.
macro_rules! readaccessorref {
    ($($RdAc:ident),*) => {
        $(  
            pub struct $RdAc<T,State>(ReadAccessor<T,State>);

            impl<T,State> $RdAc<T,State> {
                pub fn new(obj:ReadAccessor<T,State>) -> Self {
                    Self(obj)
                }
            } 

            impl<T,State> Deref for $RdAc<T,State> {
                type Target = ReadAccessor<T,State>;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<T,State> DerefMut for $RdAc<T,State> {    
            
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        )*        
    };
}

readaccessorref!(PrimaryAccessor, SecondaryAccessor);


/// QueuePtr is being used to account for the heap memory allocation for the Queue and ensure the same is dropped at the end 
/// when the drop is called.
/// QueuePtr is purposefully kept private and inaccessible. Using Deref and DerefMut the access to the same is disguised
/// within the ReaderAccessors.
struct QueuePtr<T,State> {
    ptr:Arc<AtomicPtr<LimitAccessQueue<T,State>>>,
    #[allow(dead_code)]
    owner:Arc<LimitAccessQueue<T,State>>
}

impl<T,State> QueuePtr<T,State> {
    fn new(ptr:Arc<AtomicPtr<LimitAccessQueue<T,State>>>, owner:Arc<LimitAccessQueue<T,State>>) -> Self {
        Self {
            ptr,
            owner
        }
    }
}

impl<T,State> Deref for QueuePtr<T,State> {
    type Target = Arc<AtomicPtr<LimitAccessQueue<T,State>>>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T,State> DerefMut for QueuePtr<T,State> {    
    
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ptr
    }
}

impl<T,State> Drop for QueuePtr<T,State> {
    fn drop(&mut self) {

        self.ptr.store(std::ptr::null_mut(), std::sync::atomic::Ordering::Release);
    }
}

#[derive(PartialEq)]
pub enum ReadAccessorType {
    Primary,
    Secondary
}

pub struct ReadAccessor<T,State> 
{
    val: QueuePtr<T,State>,
    rtype: ReadAccessorType,
}

#[allow(dead_code)]
impl<T,State> ReadAccessor<T,State> 
where State: Clone + Default
{
    pub fn new(owner:Arc<LimitAccessQueue<T,State>>, rtype:ReadAccessorType) -> Self {
        let arc_ptr = Arc::as_ptr(&owner) as *mut LimitAccessQueue<T,State>;
        let obj = Arc::new(AtomicPtr::new(arc_ptr));
        let val = QueuePtr::new(obj, owner);
        Self {
            val,
            rtype
        }
    }

    fn as_ptr(&self) -> Option<*mut LimitAccessQueue<T,State>> {        
        let ptr = self.val.load(std::sync::atomic::Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(ptr)                                  
        }           
    }

    fn get_ref(&self) -> Option<&LimitAccessQueue<T,State>> {
        unsafe {  
            if let Some(ptr_ref) = self.as_ptr() {
                let opt_ptr = (ptr_ref).as_ref();
                if let Some(ptr) = opt_ptr {
                    return Some(ptr);
                }                
            }          
            None
        }       
    }

    fn get_mut(&self) -> Option<&mut LimitAccessQueue<T,State>> {
        unsafe {  
            if let Some(ptr_ref) = self.as_ptr() {
                let opt_ptr = (ptr_ref).as_mut();
                opt_ptr               
            }  else {
                None
            }        
            
        }       
    }

    fn within_mutable_block<F,Output>(&self,f:F) -> Option<Output>
    where F: FnOnce(&mut LimitAccessQueue<T,State>) -> Option<Output> {

        if let Some(ptr) = self.get_mut() {
            f(ptr)                       
        } else {
            None
        }        
    }       

    pub fn is_primary(&self) -> bool {
        self.rtype == ReadAccessorType::Primary
    }

    pub fn pop(&self) -> Option<T> {  
        self.within_mutable_block(|l| l.pop())                                                                     
    }

    pub fn is_empty(&self) -> bool {
        if let Some(val) = self.within_mutable_block(|l| Some(l.is_empty())){
            val
        } else {
            true
        }
    }

    pub fn len(&self) -> usize {
        if let Some(val) = self.within_mutable_block(|l| Some(l.val.len())){
            val
        } else {
            0usize
        }
    }


    pub fn write(&self, values:Vec<T>) -> Result<bool,bool> {
        if let Some(_) = self.within_mutable_block(|l| Some(l.write(values))) {
            Ok(true)
        } else {
            Err(false)
        }        
    }

    pub fn replace(&self, values:Vec<T>) -> Result<bool,bool> {
        if let Some(_) = self.within_mutable_block(|l| Some(l.replace(values))) {
            Ok(true)
        } else {
            Err(false)
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
                
                self.within_mutable_block(|l| l.steal())               
            }
        }
    }

    pub fn steal_half(&mut self) -> Option<Vec<T>> {
        match self.rtype {
            ReadAccessorType::Secondary => {
                None
            }
            ReadAccessorType::Primary => { 
                
                self.within_mutable_block(|l| l.steal_half())               
            }
        }
    }

    pub fn set_state(&mut self, state:State) {
        self.within_mutable_block(|l| 
            Some(l.set_state(state))
        );
    }

    pub fn state(&mut self) -> State {
        if let Some(state) = self.within_mutable_block(|l| Some(l.get_state())) {
            state
        } else {
            State::default()
        }
    }
    
}