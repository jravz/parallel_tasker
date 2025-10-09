use std::sync::{Arc, RwLock};
use std::thread::Scope;
use std::sync::mpsc::{sync_channel as channel, SyncSender, TrySendError};
use arc_swap::ArcSwap;

use crate::collector::Collector;
use crate::prelude::AtomicIterator;

use super::worker_thread::{CMesg, Coordination, MessageValue, WorkerThread};

pub struct WorkerController;
type ThreadInfo<'scope, T,V> = (WorkerThread<'scope,T>,SyncSender<CMesg<V>>);

impl WorkerController
{
    pub fn run<F,V,T,C,I>(f:F, mut values:I) -> C
    where F: Fn(V) -> T + Send + Sync,
    V: Send + Sync,
    T: Send + Sync,
    C: Collector<T>,
    I:AtomicIterator<AtomicItem = V> + Send + Sized 
    {                
        let buf_size = 1;              

        std::thread::scope(            
            |s: &Scope<'_, '_>| {                   
                let mut results = C::initialize();           
                let mut threads:Vec<ThreadInfo<'_,T,V>> =Vec::new();
                let (sender, receiver) = channel::<CMesg<V>>(buf_size);
                let arc_f: Arc<RwLock<F>> = Arc::new(RwLock::new(f));                
                let arc_f_clone: Arc<RwLock<F>> = arc_f.clone();                                                          
                
                let tm = std::time::Instant::now();                                 
                if let Some(t) = WorkerThread::launch(s,String::from("Raman"),receiver,arc_f_clone) {  
                    t.unpark();                  
                    threads.push((t,sender));
                }
                let mut control_time = tm.elapsed().as_nanos();
                
                let mut vec:Option<Vec<V>>;
                let mut task:CMesg<V>;  
                let mut fails=0;                                
                let mut pos:usize = 0;

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
                // println!("Prep works = {}",tm.elapsed().as_nanos());
                let tm = std::time::Instant::now();
                let mut monitor_time = std::time::Instant::now();
                loop {                                      
                    let thread = &mut threads[pos];
                    match thread.1.try_send(task) {
                        Ok(_) => {
                            fails = 0;  
                            monitor_time = std::time::Instant::now();                          
                            vec = values.atomic_pull();                    
                            if vec.is_none() { break; }
                            task = CMesg {
                                msgtype:Coordination::Run,                            
                                msg: Some(MessageValue::Queue(vec.unwrap())), 
                            };
                        }
                        Err(e) => {
                            if let TrySendError::Full(mesg) = e {
                                task = mesg;
                                fails += 1;                                    
                            } else {
                                task = CMesg {
                                    msgtype:Coordination::Done,
                                    msg: None
                                };
                            }
                        }
                    }                    
                    
                    let elapsed_monitored_time = monitor_time.elapsed().as_nanos();
                    if fails >= threads.len() && ( elapsed_monitored_time as f64 > (control_time as f64))
                    {                        
                        if threads.len() < crate::utils::max_threads() {
                            let tm = std::time::Instant::now();
                            let (sender, receiver) = channel::<CMesg<V>>(buf_size);
                            let arc_f_clone: Arc<RwLock<F>> = arc_f.clone(); 
                            let tname = format!("T{}",threads.len());
                            if let Some(t) = WorkerThread::launch(s,tname,receiver,arc_f_clone) {                                                    
                                threads.push((t,sender));
                                // println!("New thread:{}-{}-{}",threads.len(),tm.elapsed().as_nanos(),control_time);
                                fails = 0;
                                control_time = tm.elapsed().as_nanos();
                                monitor_time = std::time::Instant::now();
                            }
                        }
                    } 

                    pos += 1;
                    if pos >= threads.len() { pos = 0; }                   
                }                   
                println!("Cycles done = {}",tm.elapsed().as_nanos());

                let tm = std::time::Instant::now();
                for (_,sender) in &mut threads {   
                    let tm = std::time::Instant::now();
                    let mut done_task = CMesg {
                        msgtype:Coordination::Done,
                        msg: None
                    };          
                    while sender.try_send(done_task).is_err() {                        
                        done_task = CMesg {
                            msgtype:Coordination::Done,
                            msg: None
                        };
                    }
                    // println!("part 1 = {}",tm.elapsed().as_nanos());
                }
                for (thread,_) in threads {                                              
                    let tm = std::time::Instant::now();     
                    if let Ok(res) = thread.close(){
                        results.extend(res.into_iter());
                    };  
                    // println!("part 2 = {}",tm.elapsed().as_nanos());                                                                                    
                }  
                println!("Final = {}",tm.elapsed().as_nanos());
                results                                                                            
            }
        )
    }
}

