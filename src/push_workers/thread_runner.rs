use std::sync::{mpsc::{Receiver, Sender}, Arc, RwLock};
 use chrono::{Local, DateTime};

use crate::{accessors::read_accessor::{ReadAccessor, SecondaryAccessor}, push_workers::worker_thread::{CMesg, Coordination, MessageValue, ThreadMesg, ThreadShare, ThreadState}, utils::SpinWait};

pub struct ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T
{
    receiver:Receiver<CMesg<V>>,
    thread_state:Arc<RwLock<ThreadShare<V>>>,
    sender:Sender<ThreadMesg>, 
    pos:usize, 
    f:Arc<RwLock<F>>,
    secondary_q:SecondaryAccessor<V,Coordination>,
    buf_size: usize
}

impl<F,V,T> ThreadRunner<F,V,T> 
where T:Send,
V:Send,
F:Fn(V) -> T {

    pub fn new(receiver:Receiver<CMesg<V>>, thread_state:Arc<RwLock<ThreadShare<V>>>,
        sender:Sender<ThreadMesg>, pos:usize, buf_size:usize, secondary_q:SecondaryAccessor<V,Coordination>, 
        f:Arc<RwLock<F>>) -> Self 
    {

        Self {
            receiver,
            thread_state,
            sender,
            pos,
            f,
            secondary_q,
            buf_size
        }

    }    

    fn process(&mut self, final_values:&mut Vec<T>, processed:&mut usize) 
    {
        let fread: std::sync::RwLockReadGuard<'_, F> = self.f.read().unwrap();
        SpinWait::loop_while_mut(||self.secondary_q.is_empty());
        *processed += self.secondary_q.len();
        while let Some(value) = self.secondary_q.pop() {                                        
            final_values.push(fread(value));
        }
        self.secondary_q.set_state(Coordination::Waiting);                                                                                                                                                                                                          
    }

    pub fn run(&mut self) -> Vec<T> {
        let mut final_values:Vec<T> = Vec::new();                     
        let mut processed = 0;
        let mut waiting_time:u128 = 0;
        let mut processing_time:u128 = 0;
        let total_time = std::time::Instant::now();
        let mut vec_process_start:Vec<String> = Vec::new();
        let mut vec_wait_start:Vec<String> = Vec::new();
        loop 
        {                                    
            match self.secondary_q.state() {                
                Coordination::Park => {
                    std::thread::park();
                },
                Coordination::Run => {
                    vec_process_start.push(Local::now().format("%H:%M:%S::%.9f").to_string());
                    let tm = std::time::Instant::now();                                                    
                    self.process(&mut final_values, &mut processed);   
                    processing_time += tm.elapsed().as_nanos(); 
                    vec_wait_start.push(Local::now().format("%H:%M:%S::%.9f").to_string());                                                                             
                },
                Coordination::Done => {    
                    // println!("Received Done: {}",self.pos);                        
                    break;
                },
                Coordination::Unwind => {
                    panic!("There was some error.");
                },
                Coordination::Panic => {
                    panic!("There was some error.");
                }, 
                Coordination::Waiting => {  
                    let tm = std::time::Instant::now();                  
                    SpinWait::loop_while_mut(||self.secondary_q.state() == Coordination::Waiting);
                    waiting_time += tm.elapsed().as_nanos();
                }                                             
                _ => {}
            }            
        }
        let tot_time = total_time.elapsed().as_nanos();
        println!("{}: totaltime: {}| processed = {}",self.pos, tot_time,processed);
        println!("{}: processingtime: {}, % of total:{}%",self.pos, processing_time,((processing_time as f64 /tot_time as f64)*10000.0 as f64).round()/100.0);
        println!("{}: waitingtime: {}, % of total:{}%",self.pos, waiting_time,((waiting_time as f64 /tot_time as f64)*10000.0 as f64).round()/100.0);                          
        for idx in 0..vec_process_start.len() {
            println!("{}: [{}] -- [{}]",self.pos, vec_process_start[idx], vec_wait_start[idx]);
        }
        final_values       
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

}   