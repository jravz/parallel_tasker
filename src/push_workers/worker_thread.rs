//! Individual worker thread that is spawned by the workercontroller and thereon managed by
//! the thread manager

use std::{any::Any, error::Error, sync::{Arc, RwLock}};

use crate::{accessors::{limit_queue, read_accessor::{PrimaryAccessor, SecondaryAccessor}}, errors::WorkThreadError, push_workers::thread_runner::ThreadRunner, utils::SpinWait};


/// Coordination is used as a State variable by the Primary and Secondary Accessors to manage the 
/// state of the thread
#[repr(u8)]
#[derive(Clone,Debug,PartialEq,Default)]
pub enum Coordination
{
    #[default]
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


/// Manage specific stats about the tasks in operation to enable the scheduling algorithm
/// to take specific decisions
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

    /// Lets assume the least case. If the queue had 1 task and had been running for 1 microsecond,
    /// the time elapsed on average for a task is 1microsecond. Now if there were two tasks and the second task has been running for 0.5 microsecond,
    /// the avg time is 1.5/2 = 0.75 microsecond. The time to finish may be 0.5, but the system conservatively calculates
    /// as avg time * remaining = 0.75microsecond. This becomes more accurate for 100+ tasks
    pub fn time_per_task(&self, curr_len:usize) -> f64 {
        self.elapsed_time() as f64 / (self.initial_queue_len() - curr_len + 1) as f64               
    }

    pub fn ratio_of_tasks_remaining(&self, curr_len:usize) -> f64 {
        if self.initial_queue_len() == 0 {
            return 0.0;
        }
        curr_len as f64 / self.initial_queue_len() as f64
    }
}


/// Worker thread is launched by the thread manager based on the need discovered by the 
/// scheduling algorithm within worker controller.
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

    /// Run function runs a new batch of tasks on the thread
    pub fn run(&mut self, values:Vec<V>) -> Result<(), WorkThreadError> {
        if values.is_empty() {
            Err(WorkThreadError::Other("Values within task shared to queue were empty.".to_owned()))
        } else {
            self.queue_stats = Some(QueueStats::new(values.len(), std::time::Instant::now()));
            self.primary_q.replace(values).map_err(|_|WorkThreadError::Other("Unknown error occured.".to_owned()))?;            
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

    pub fn ratio_of_tasks_remaining(&self) -> Option<f64> {
        self.queue_stats.as_ref().map(|q|
        q.ratio_of_tasks_remaining(self.primary_q.len()))
    }


    // The user is asked to share a min_ratio_completed as the projection may not be reliable for lower
    // ratios of completion. If that ratio is not completed, then the system returns None
    pub fn projected_time_for_completion(&self, min_ratio_completed:f64) -> Option<f64> {
        if let Some(ratio_pending) = self.ratio_of_tasks_remaining() {
            if ratio_pending < (1.0 - min_ratio_completed) {
                return Some(self.time_per_process().unwrap() * ratio_pending * self.queue_len() as f64);
            } else {
                return None;
            }
        }
        None
    }

}