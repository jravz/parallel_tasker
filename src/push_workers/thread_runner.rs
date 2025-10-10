use std::sync::{mpsc::{Receiver, Sender}, Arc, RwLock};

use crate::push_workers::worker_thread::{CMesg, Coordination, MessageValue, ThreadMesg, ThreadShare, ThreadState};

pub struct ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T
{
    receiver:Receiver<CMesg<V>>,
    thread_state:Arc<RwLock<ThreadShare>>,
    sender:Sender<ThreadMesg>, 
    pos:usize, 
    f:Arc<RwLock<F>>
}

impl<F,V,T> ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T {

    pub fn new(receiver:Receiver<CMesg<V>>, thread_state:Arc<RwLock<ThreadShare>>,
        sender:Sender<ThreadMesg>, pos:usize, f:Arc<RwLock<F>>) -> Self {

        Self {
            receiver,
            thread_state,
            sender,
            pos,
            f
        }

    }

    fn process(
               &self, receipt:CMesg<V>, final_values:&mut Vec<T>, processed:&mut usize,
               fread: &std::sync::RwLockReadGuard<'_, F>
    ) {
        if let Some(values) = receipt.msg {                                                        
            let mut writer = self.thread_state.write().unwrap(); 
            writer.state = ThreadState::Busy;
            drop(writer);
            if let MessageValue::Queue(values) = values { 
                *processed = *processed + values.len();                                                             
                for val in values {                                    
                    final_values.push(fread(val));
                }                                
                let mut writer = self.thread_state.write().unwrap();                                 
                writer.state = ThreadState::Done;
                drop(writer);  
                _ = self.sender.send(ThreadMesg::Free(self.pos, std::time::Instant::now()));                                                                                                                              
            }                            
        }
    }

    pub fn run(&mut self) -> Vec<T> {
        let mut final_values:Vec<T> = Vec::new();     
        let fread: std::sync::RwLockReadGuard<'_, F> = self.f.read().unwrap();
        _ = self.sender.send(ThreadMesg::Free(self.pos, std::time::Instant::now()));
        let mut wait_tm_instant = std::time::Instant::now();
        let mut waittime = 0;
        let mut processtime = 0;
        let mut processed = 0;
        loop 
        {            
            if let Ok(receipt) = self.receiver.recv() {
                match receipt.msgtype {
                    Coordination::Park => {
                        std::thread::park();
                    },
                    Coordination::Run => {                                  
                        waittime += wait_tm_instant.elapsed().as_nanos();            
                        let processing = std::time::Instant::now();
                        self.process(receipt, &mut final_values, &mut processed, &fread);
                        processtime += processing.elapsed().as_nanos();                                
                        wait_tm_instant = std::time::Instant::now();                                               
                    },
                    Coordination::Done => {    
                        _ = self.sender.send(ThreadMesg::Stopped(self.pos, std::time::Instant::now()));                                         
                        break;
                    },
                    Coordination::Unwind => {
                        panic!("There was some error.");
                    },
                    Coordination::Panic => {
                        panic!("There was some error.");
                    },
                    Coordination::ProcessTime => {
                        _ = self.sender.send(ThreadMesg::Time(self.pos, processtime));                                         
                    },
                    Coordination::WaitTime => {
                        _ = self.sender.send(ThreadMesg::Time(self.pos, waittime));                                         
                    },
                    Coordination::Processed => {
                        _ = self.sender.send(ThreadMesg::Quantity(self.pos, processed));                                         
                    },
                    _ => {}
                }
            }
        }         
        drop(fread);
        final_values       
    }

}