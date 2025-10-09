use std::{any::Any, sync::{mpsc::Receiver, Arc, RwLock}};
use arc_swap::ArcSwap;
pub trait ThreadJob {
    type Worker;
    type JobOutput;
    type JobInput;

    fn run_job(&mut self) -> Self::JobOutput;
}

#[repr(u8)]
#[derive(Clone)]
pub enum Coordination
{
    Run=0,
    Park=1,
    Done=2,
    Unwind=3,
    Panic=4
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

pub struct ThreadShare {
    pub state: ThreadState,
}

impl ThreadShare {
    pub fn new() -> Self {        
        Self {
            state: ThreadState::New,            
        }
    }
}

pub struct WorkerThread<'scope,T> {
    pub thread:Option<std::thread::ScopedJoinHandle<'scope,Vec<T>>>,
    pub name:String,
    pub state:Arc<RwLock<ThreadShare>>
}

impl<'scope,T> WorkerThread<'scope,T> 
where T:Send + Sync + 'scope
{

    pub fn launch<'env,'a, V,F>(scope: &'scope std::thread::Scope<'scope, 'env>,name:String,
    receiver:Receiver<CMesg<V>>,f:ArcSwap<F>) -> Option<Self> 
    where 'env: 'scope,    
    V:Send + Sync + 'scope,
    F:Fn(V) -> T + Send + Sync + 'scope
    {

        // let (th_sender, th_receiver) = channel::<Vec<T>>();
        
        let thread_name = name.clone();

        let thread_state: Arc<RwLock<ThreadShare>> = Arc::new(RwLock::new(ThreadShare::new()));
        let state_clone: Arc<RwLock<ThreadShare>> = thread_state.clone();

        let scoped_thread: std::thread::ScopedJoinHandle<'_, Vec<T>> = std::thread::Builder
                            ::new()
                            .name(name)
                            .spawn_scoped(scope, move || Self::task_loop(receiver,state_clone,f)).unwrap();                            


        let worker = WorkerThread {
            name:thread_name, 
            thread: Some(scoped_thread),
            state: thread_state                     
        };        

        Some(worker)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn unpark(&self) {               
        self.thread.as_ref()
        .iter().for_each(|t| t.thread().unpark());
    }

    pub fn task_loop<V,F>(receiver:Receiver<CMesg<V>>, thread_state:Arc<RwLock<ThreadShare>>,f:ArcSwap<F>) -> Vec<T>
    where T:Send,
    V:Send,
    F:Fn(V) -> T
    {   
        let mut final_values:Vec<T> = Vec::new();     
        let fread = f.load();        
        loop 
        {            
            if let Ok(receipt) = receiver.recv() {
                match receipt.msgtype {
                    Coordination::Park => {
                        std::thread::park();
                    },
                    Coordination::Run => {                                               
                        if let Some(values) = receipt.msg {
                            let mut writer = thread_state.write().unwrap(); 
                            writer.state = ThreadState::Busy;
                            drop(writer);
                            if let MessageValue::Queue(values) = values {                                
                                for val in values {                                    
                                    final_values.push(fread(val));
                                }
                                let mut writer = thread_state.write().unwrap();                                 
                                writer.state = ThreadState::Done;
                                drop(writer);                                     
                            }                            
                        }                                                
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
                }
            }
        } 

        final_values       
    }

    pub fn state(&self) -> ThreadState {
        let val = self.state.read().unwrap().state.clone();
        val        
    }

    pub fn close(self) -> Result<Vec<T>, Box<dyn Any + Send + 'static>> {
        
        self.thread.unwrap().join()   

    }


}