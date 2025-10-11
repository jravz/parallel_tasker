use std::{any::Any, sync::{mpsc::{sync_channel, Receiver, Sender, SyncSender}, Arc, RwLock}, time::Instant};

use crate::push_workers::thread_runner::ThreadRunner;

pub enum ThreadMesg {
    Free(usize,Instant),
    Stopped(usize,Instant),
    Time(usize, u128),
    Quantity(usize, usize)    
}

#[repr(u8)]
#[derive(Clone)]
pub enum Coordination
{
    Run=0,
    Park=1,
    Done=2,
    Unwind=3,
    Panic=4,
    Ignore=5,
    ProcessTime=6,
    WaitTime=7,
    Processed=8
}

#[derive(Clone)]
pub enum MessageValue<V> {
    Queue(Vec<V>),
    Text(String)
}

#[derive(Clone)]
pub struct CMesg<V>
where V:Send 
{
    pub msgtype: Coordination,
    pub msg: Option<MessageValue<V>>
}

#[derive(Debug,Clone,PartialEq)]
#[repr(u8)]
pub enum ThreadState {
    New=0,
    Busy=1,
    Done=2,
    Park=3
}

pub struct ThreadShare<V> {
    pub state: ThreadState,
    pub queue:Vec<V>    
}

impl<V> ThreadShare<V> {
    pub fn new() -> Self {        
        Self {
            state: ThreadState::New, 
            queue: Vec::new()
        }
    }
}

pub struct WorkerThread<'scope,V,T> 
where V:Send
{
    pub thread:Option<std::thread::ScopedJoinHandle<'scope,Vec<T>>>,
    pub name:String,
    pub state:Arc<RwLock<ThreadShare<V>>>,
    pos: usize,
    sender: SyncSender<CMesg<V>>,
    buf_size: usize
}

impl<'scope,V,T> WorkerThread<'scope,V,T> 
where T:Send + Sync + 'scope,
V:Send + Sync + 'scope
{

    pub fn launch<'env,'a,F>(scope: &'scope std::thread::Scope<'scope, 'env>,
    work_sender:Sender<ThreadMesg>, pos:usize, buf_size:usize, f:Arc<RwLock<F>>) -> Option<Self> 
    where 'env: 'scope,    
    V:Send + Sync + 'scope,
    F:Fn(V) -> T + Send + Sync + 'scope
    {                
        let thread_name = format!("T:{}",pos);        
        let thread_state: Arc<RwLock<ThreadShare<V>>> = Arc::new(RwLock::new(ThreadShare::new()));
        let state_clone: Arc<RwLock<ThreadShare<V>>> = thread_state.clone();
        let (sender, receiver) = sync_channel::<CMesg<V>>(buf_size);

        let scoped_thread: std::thread::ScopedJoinHandle<'_, Vec<T>> = std::thread::Builder
                            ::new()
                            .name(thread_name.clone())
                            .spawn_scoped(scope, move || Self::task_loop(receiver,state_clone,work_sender, pos, buf_size, f)).unwrap();                                    

        let worker = WorkerThread {
            name:thread_name, 
            thread: Some(scoped_thread),
            state: thread_state ,
            pos,
            sender ,
            buf_size                   
        };        

        Some(worker)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn send(&mut self, task:CMesg<V>) -> Result<(), std::sync::mpsc::SendError<CMesg<V>>> {
        self.sender.send(task)
    }

    pub fn try_send(&mut self, task:CMesg<V>) -> Result<(), std::sync::mpsc::TrySendError<CMesg<V>>> {
        self.sender.try_send(task)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn unpark(&self) {               
        self.thread.as_ref()
        .iter().for_each(|t| t.thread().unpark());
    }

    fn done(&mut self) -> Result<(), std::sync::mpsc::SendError<CMesg<V>>> {
        self.send(
            CMesg { msgtype: Coordination::Done, msg: None }
        )
    }    

    fn task_loop<F>(receiver:Receiver<CMesg<V>>, thread_state:Arc<RwLock<ThreadShare<V>>>,
                    sender:Sender<ThreadMesg>, pos:usize, buf_size:usize, f:Arc<RwLock<F>>) -> Vec<T>
    where T:Send,
    V:Send,
    F:Fn(V) -> T
    {   
        ThreadRunner::new(receiver,thread_state,sender,pos,buf_size, f)
        .run()
    }

    pub fn state(&self) -> ThreadState {
        let val = self.state.read().unwrap().state.clone();
        val        
    }

    pub fn join(mut self) -> Result<Vec<T>, Box<dyn Any + Send + 'static>> 
    where V:Send + Sync + 'scope,
    {
        while self.done().is_err() {
        }
        self.thread.unwrap().join()   

    }


}