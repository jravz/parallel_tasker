use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel, Receiver, Sender, TrySendError};

use crate::collector::Collector;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::{CMesg, Coordination, MessageValue, WorkerThread};

// With 2 queues, the thread always has a backup to work on and does not wait
const BUF_SIZE_CHANNEL:usize = 2;

pub struct WorkerController<F,V,T,I> 
where F: Fn(V) -> T + Send + Sync,
V: Send + Sync,
T: Send + Sync,    
I:AtomicIterator<AtomicItem = V> + Send + Sized 
{
    f:Arc<RwLock<F>>,
    values: I,
    all_sender:Sender<ThreadMesg>,
    all_receiver: Receiver<ThreadMesg>,
    buf_size: usize
}

type ThreadInfo<'scope, T,V> = WorkerThread<'scope,V,T>;

impl<F,V,T,I>  WorkerController<F,V,T,I>
where F: Fn(V) -> T + Send + Sync,
V: Send + Sync,
T: Send + Sync,    
I:AtomicIterator<AtomicItem = V> + Send + Sized  
{

    pub fn new(f:F, values:I) -> Self 
    {
        //send/receive channel to reply on success
        let (all_sender, all_receiver) = async_channel::<ThreadMesg>();
        
        Self {
            f: Arc::new(RwLock::new(f)),
            values: values,
            all_sender,
            all_receiver,
            buf_size: BUF_SIZE_CHANNEL
        }
    }

    pub fn run<C>(&mut self) -> C
    where C: Collector<T>,    
    {                        
        let max_threads = crate::utils::max_threads();              

        std::thread::scope(            
            |s: &Scope<'_, '_>| {                   
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                                                                                                                                                     
                
                // Generate first worker thread as you need 1 by default. Record control time
                let tm = std::time::Instant::now();                                                                                               
                let worker_sender_clone = self.all_sender.clone();                                                         
                let arc_f_clone: Arc<RwLock<F>> = self.f.clone();                             
                if let Some(t) = WorkerThread::launch(s,worker_sender_clone,threads.len(),self.buf_size, arc_f_clone) {                                                    
                    threads.push(t);                                                                                                          
                }
                let mut control_time = tm.elapsed().as_nanos();                          
                
                let vec:Option<Vec<V>>;
                let mut task:CMesg<V>;  
                let mut fails=0;                                

                vec = self.values.atomic_pull();                                   
                task = if let Some(vec) = vec {
                    CMesg {
                    msgtype:Coordination::Run,                            
                    msg: Some(MessageValue::Queue(vec))} 
                } else {
                    CMesg {
                    msgtype:Coordination::Done,                            
                    msg: None } 
                };  
                           
                let mut process_time = std::time::Instant::now(); 
                let mut elapsed_monitored_time = 0;
                loop {                                                                                                              
                    if let Ok(msg) = self.all_receiver.try_recv() {
                        if let ThreadMesg::Free(pos, _) = msg {                            
                            elapsed_monitored_time = process_time.elapsed().as_nanos();                        
                            process_time = std::time::Instant::now();                      
                            let thread= &mut threads[pos];                        
                            task = if let Some((task,status)) = Self::send_task(thread, &mut self.values, task) {
                                if !status { fails += 1; } 
                                task
                            } else {
                                break;
                            };
                        }                                                                         
                    } 
 
                    if threads.len() < max_threads { 
                        if elapsed_monitored_time as f64 > control_time as f64 {
                            let tm = std::time::Instant::now();
                            let worker_sender_clone = self.all_sender.clone();                                                         
                            let arc_f_clone: Arc<RwLock<F>> = self.f.clone();                             
                            if let Some(t) = WorkerThread::launch(s,worker_sender_clone,threads.len(),self.buf_size, arc_f_clone) {                                                    
                                threads.push(t);                                
                                fails = 0;                                                             
                            }
                            control_time = tm.elapsed().as_nanos();                                
                        }                            
                    }                                                                                                                                                                                                                
                }                                                   
                                
                for thread in threads {                                                                    
                    if let Ok(res) = thread.join(){
                        results.extend(res.into_iter());
                    };                                                                                                     
                }                      
                       
                results                                                                            
            }
            
        )
    }

    fn send_task<'scope>(thread:&mut WorkerThread<'scope, V,T>,
    values:&mut I, task:CMesg<V>) -> Option<(CMesg<V>,bool)>
    where V: Send + Sync + 'scope,
    T: Send + Sync + 'scope,    
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {                
        let task = match thread.try_send(task) {
            Ok(_) => {                   
                let vec = values.atomic_pull();                                                   
                if vec.is_none() { return None; }
                (CMesg {
                    msgtype:Coordination::Run,                            
                    msg: Some(MessageValue::Queue(vec.unwrap())), 
                },true)
            }
            Err(e) => {                
                if let TrySendError::Full(mesg) = e {
                    (mesg,false)
                } else {
                    (CMesg {
                        msgtype:Coordination::Done,
                        msg: None
                    },false)
                }
            }
        };        
        
        Some(task)
    }
}

