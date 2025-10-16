use std::{any::Any, error::Error, sync::{mpsc::Sender, Arc, RwLock}, time::Instant};

use crate::{accessors::{limit_queue, read_accessor::{PrimaryAccessor, SecondaryAccessor}}, push_workers::thread_runner::ThreadRunner, utils::SpinWait};

pub enum ThreadMesg {
    Free(usize,Instant),
    Stopped(usize,Instant),
    Time(usize, u128), 
    Quantity(usize, usize)    
}

#[repr(u8)]
#[derive(Clone,Debug,PartialEq)]
pub enum Coordination
{
    Waiting=0,
    Park=1,
    Done=2,
    Unwind=3,
    Panic=4,
    Ignore=5,
    ProcessTime=6,
    Run=7,
    Processed=8
}

impl Default for Coordination {
    fn default() -> Self {
        Coordination::Waiting
    }
}

#[derive(Clone,Debug)]
pub enum MessageValue<V> {
    Queue(Vec<V>),
    Text(String)
}

#[derive(Clone,Debug)]
pub struct CMesg<V>
where V:Send 
{
    pub msgtype: Coordination,
    pub msg: Option<MessageValue<V>>
}

impl<V> CMesg<V>
where V:Send  {
    pub fn done() -> Self {
        Self {
            msgtype: Coordination::Done,
            msg: None
        }
    }

    pub fn run_task() -> Self {
        Self {
            msgtype: Coordination::Run,
            msg: None
        }
    }
    
}

pub struct QueueStats {
    process_time: std::time::Instant,
    start_queue_len: usize
}

impl QueueStats {
    pub fn new(start_queue_len:usize, process_time: std::time::Instant) -> Self {
        Self {
            process_time,
            start_queue_len
        }
    }

    pub fn initial_queue_len(&self) -> usize {
        self.start_queue_len
    }

    pub fn elapsed_time(&self) -> u128 {
        self.process_time.elapsed().as_nanos()
    }

    pub fn time_per_task(&self, curr_len:usize) -> f64 {
        if self.initial_queue_len() == 0 {
            1.0
        } else {
            self.elapsed_time() as f64 /
            (self.initial_queue_len() - curr_len) as f64            
        }
    }
}

#[allow(dead_code)]
pub struct WorkerThread<'scope,V,T> 
where V:Send
{
    pub thread:Option<std::thread::ScopedJoinHandle<'scope,Vec<T>>>,
    pub name:String,    
    pos: usize,    
    pub primary_q: PrimaryAccessor<V,Coordination>,
    queue_stats:  Option<QueueStats>
}

impl<'scope,V,T> WorkerThread<'scope,V,T> 
where T:Send + Sync + 'scope,
V:Send + Sync + 'scope
{

    pub fn launch<'env,'a,F>(scope: &'scope std::thread::Scope<'scope, 'env>,
    pos:usize,  f:Arc<RwLock<F>>) -> Result<Self,Box<dyn Error>> 
    where 'env: 'scope,    
    V:Send + Sync + 'scope,
    F:Fn(V) -> T + Send + Sync + 'scope
    {                
        let thread_name = format!("T:{}",pos);                              
        let (primary_q, secondary_q) = limit_queue::LimitAccessQueue::<V,Coordination>::new();

        match std::thread::Builder
        ::new()
        .name(thread_name.clone())
        .spawn_scoped(scope, move || Self::task_loop(pos, secondary_q, f)) {
            Ok(scoped_thread) => {
                let worker = WorkerThread {
                    name:thread_name, 
                    thread: Some(scoped_thread),                    
                    pos,                    
                    primary_q,
                    queue_stats: None                              
                };
                Ok(worker)
            }
            Err(e) => {
                Err(Box::new(e))
            }
        }                            
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn signal(&mut self, state:Coordination) {
        self.primary_q.set_state(state);
    }

    pub fn is_running(&mut self) -> bool {
        self.primary_q.state() == Coordination::Run
    }

    pub fn run(&mut self, values:Vec<V>) -> Result<(), ()> {
        if values.is_empty() {
            Err(())
        } else {
            self.queue_stats = Some(QueueStats::new(values.len(), std::time::Instant::now()));
            self.primary_q.replace(values).map_err(|_|())?;            
            self.signal(Coordination::Run);
            Ok(())
        }        
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn unpark(&self) {               
        self.thread.as_ref()
        .iter().for_each(|t| t.thread().unpark());
    }

    fn done(&mut self) {                
        SpinWait::loop_while_mut(||!self.primary_q.is_empty() || (self.primary_q.state() != Coordination::Waiting));        
        self.primary_q.set_state(Coordination::Done);
    }    

    fn task_loop<F>(pos:usize, secondary_q:SecondaryAccessor<V,Coordination>, f:Arc<RwLock<F>>) -> Vec<T>
    where T:Send,
    V:Send,
    F:Fn(V) -> T
    {   
        ThreadRunner::new(pos,secondary_q, f)
        .run()
    }

    pub fn join(mut self) -> Result<Vec<T>, Box<dyn Any + Send + 'static>> 
    where V:Send + Sync + 'scope,
    {        
        self.done();     
        self.thread.unwrap().join()                
    }

    pub fn queue_len(&self) -> usize {
        self.primary_q.len()
    }   

    pub fn get_elapsed_time(&mut self) -> Option<u128> {
        self.queue_stats.as_ref().map(QueueStats::elapsed_time)
    }

    pub fn time_per_process(&self) -> Option<f64> {
        self.queue_stats.as_ref().map(|q|
        q.time_per_task(self.primary_q.len()))        
    }

}