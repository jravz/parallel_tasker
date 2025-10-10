use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{channel as async_channel,TrySendError};

use crate::collector::Collector;
use crate::prelude::AtomicIterator;
use crate::push_workers::worker_thread::ThreadMesg;

use super::worker_thread::{CMesg, Coordination, MessageValue, WorkerThread};
pub struct WorkerController;
type ThreadInfo<'scope, T,V> = WorkerThread<'scope,V,T>;

impl WorkerController
{
    pub fn run<F,V,T,C,I>(f:F, mut values:I) -> C
    where F: Fn(V) -> T + Send + Sync,
    V: Send + Sync,
    T: Send + Sync,
    C: Collector<T>,
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {                
        let buf_size = 2;     
        let max_threads = crate::utils::max_threads();              

        std::thread::scope(            
            |s: &Scope<'_, '_>| {                   
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();                
                let arc_f: Arc<RwLock<F>> = Arc::new(RwLock::new(f));                                                                                      
                
                //send/receive channel to reply on success
                let (worker_sender, all_receiver) = async_channel::<ThreadMesg>();

                let tm = std::time::Instant::now();                                                                                               
                let worker_sender_clone = worker_sender.clone();                                                         
                let arc_f_clone: Arc<RwLock<F>> = arc_f.clone();                             
                if let Some(t) = WorkerThread::launch(s,worker_sender_clone,threads.len(),buf_size, arc_f_clone) {                                                    
                    threads.push(t);                                                                                                          
                }
                let mut control_time = tm.elapsed().as_nanos();                          
                
                let vec:Option<Vec<V>>;
                let mut task:CMesg<V>;  
                let mut fails=0;                                

                vec = values.atomic_pull();                                   
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
                    if let Ok(msg) = all_receiver.try_recv() {
                        if let ThreadMesg::Free(pos, _) = msg {                            
                            elapsed_monitored_time = process_time.elapsed().as_nanos();                        
                            process_time = std::time::Instant::now();                      
                            let thread= &mut threads[pos];                        
                            task = if let Some((task,status)) = Self::send_task(thread, &mut values, task) {
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
                            let worker_sender_clone = worker_sender.clone();                                                         
                            let arc_f_clone: Arc<RwLock<F>> = arc_f.clone();                             
                            if let Some(t) = WorkerThread::launch(s,worker_sender_clone,threads.len(),buf_size, arc_f_clone) {                                                    
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

    fn send_task<'scope, T,V,I>(thread:&mut WorkerThread<'scope, V,T>,
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

