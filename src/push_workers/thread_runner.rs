//! ThreadRunner is responsible for running the tasks within queue belonging to each thread. It manages the process within a run function that is effectively
//! a loop. The queue itself is a LimitedAccessQueue with the Secondary Accessor being available here

use std::sync::{Arc, RwLock};

use crate::{accessors::read_accessor::SecondaryAccessor, push_workers::worker_thread::Coordination, utils::SpinWait};

pub struct ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T
{            
    pos:usize, 
    f:Arc<RwLock<F>>,
    secondary_q:SecondaryAccessor<V,Coordination>,    
}

impl<F,V,T> ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T {

    pub fn new(pos:usize, secondary_q:SecondaryAccessor<V,Coordination>, 
        f:Arc<RwLock<F>>) -> Self 
    {

        Self {                                    
            pos,
            f,
            secondary_q            
        }

    }    

    fn process(&mut self, final_values:&mut Vec<T>, processed:&mut usize) 
    {
        let fread: std::sync::RwLockReadGuard<'_, F> = self.f.read().unwrap();
        SpinWait::loop_while_mut(||self.secondary_q.is_empty());        
        while let Some(value) = self.secondary_q.pop() {            
            final_values.push(fread(value));                                              
        }
        self.secondary_q.set_state(Coordination::Waiting);                                                                                                                                                                                                          
    }

    pub fn run(&mut self) -> Vec<T> {
        let mut final_values:Vec<T> = Vec::new();                     
        let mut processed = 0;         

        loop 
        {                                    
            match self.secondary_q.state() {                
                Coordination::Park => {
                    std::thread::park();
                },
                Coordination::Run => {                                                             
                    self.process(&mut final_values, &mut processed);                        
                },
                Coordination::Done => {                      
                    break;
                },
                Coordination::Unwind => {
                    panic!("There was some error.");
                },
                Coordination::Panic => {
                    panic!("There was some error.");
                }, 
                Coordination::Waiting => {                                          
                    SpinWait::loop_while_mut(||self.secondary_q.state() == Coordination::Waiting);                     
                }                                             
                _ => {}
            }            
        }

        final_values       
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

}   