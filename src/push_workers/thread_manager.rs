//! Thread Manager encapsulates all active worker threads and information on free threads

use std::{collections::VecDeque, sync::{Arc, RwLock}};

use crate::{collector::Collector, errors::WorkThreadError, push_workers::worker_thread::WorkerThread};

pub struct ThreadManager<'env, 'scope,Input,Output,F>
where Input: Send + Sync + 'scope,
Output: Send + Sync + 'scope,  
F: Fn(Input) -> Output + Send + Sync + 'scope,
'env: 'scope
{
    threads:Vec<WorkerThread<'scope,Input,Output>>,
    scope:&'scope std::thread::Scope<'scope, 'env>,
    max_threads: usize,
    free_threads: VecDeque<usize>,
    f:  Arc<RwLock<F>>
}

impl<'env, 'scope,Input,Output,F> ThreadManager<'env, 'scope,Input,Output,F> 
where Input: Send + Sync + 'scope,
Output: Send + Sync + 'scope,
F: Fn(Input) -> Output + Send + Sync + 'scope,
'env: 'scope
{

    pub fn new(scope: &'scope std::thread::Scope<'scope, 'env>,f: Arc<RwLock<F>>, max_threads:usize) -> Self {
        Self {
            threads: Vec::new(),
            scope,
            max_threads,
            free_threads:VecDeque::new(),
            f
        }
    }

    pub fn has_free_threads(&self) -> bool {
        !self.free_threads.is_empty()
    }

    pub fn add_to_free_queue(&mut self,pos:usize) {
        self.free_threads.push_front(pos);   
    }

    pub fn clear_free_threads(&mut self) {
        self.free_threads.clear();
    }

    pub fn pop_from_free_queue(&mut self) -> Option<usize> {
        self.free_threads.pop_back()
    }

    pub fn thread_len(&self) -> usize {
        self.threads.len()
    }

    pub fn get_mut_thread(&mut self,pos:usize) -> &mut WorkerThread<'scope,Input,Output> {
        &mut self.threads[pos]
    }

    pub fn threads_as_mutable(&mut self) -> &mut Vec<WorkerThread<'scope,Input,Output>> {
        &mut self.threads
    } 

    pub fn add_thread(&mut self) -> Result<(),WorkThreadError>
    where Input: 'scope,
    Output: 'scope,
    F: Fn(Input) -> Output + Send + Sync + 'scope,
    {                                                               
        let arc_f_clone: Arc<RwLock<F>> = self.f.clone();       
        match WorkerThread::launch(self.scope,self.threads.len(), arc_f_clone) {
            Ok(t) =>  {                                                    
                self.threads.push(t);  
                Ok(())                                                                                                        
            } 
            Err(e) =>  {
                Err(WorkThreadError::ThreadAdd(e.to_string()))
            }
        }                              
    } 

    pub fn join_all_threads<C>(&mut self) -> Result<C,WorkThreadError>
    where 'env: 'scope,     
    Input: Send + Sync + 'scope,
    Output: Send + Sync + 'scope, 
    C: Collector<Output>   
    {
        let mut results = C::initialize();   
        while !self.threads.is_empty() {
            if let Some(thread) = self.threads.pop() {
                results.extend(
                thread.join()
                .map_err(|_|WorkThreadError::ThreadJoin)?
                .into_iter()
            );
            }            
        }        

        Ok(results)

    }

    pub fn refresh_free_threads(&mut self, mut control_time:u128) -> Result<u128,WorkThreadError>
    {        
        self.clear_free_threads();
        let mut count_high_process_time:usize = 0;  
        let more_threads_available = self.thread_len() < self.max_threads;     
        for idx in 0..self.thread_len() {
            let thread = self.get_mut_thread(idx);
            let pos = thread.pos();
            if !thread.is_running() && thread.is_queue_empty() {
                self.add_to_free_queue(pos);
            } 
            // Attempt to predict the time required for completion and use that to test
            // if that exceeds the time to spin out a thread
            else if more_threads_available
            {
                if let Some (processtime) = thread.time_per_process() {   
                    let predicted_queue_time = processtime * thread.queue_len() as f64;                    
                    if predicted_queue_time > control_time as f64 {
                        count_high_process_time += 1;
                    }    
                }                 
            }    
        }        

        if more_threads_available && !self.has_free_threads() && count_high_process_time > 0 {
            let new_thread_count = usize::min(self.max_threads - self.thread_len(),count_high_process_time);
            let tm = std::time::Instant::now();  
            for _ in 0..new_thread_count {
                self.add_thread()?;                                                                                
                self.add_to_free_queue(self.thread_len() - 1);  
            }  
            control_time = tm.elapsed().as_nanos() / new_thread_count as u128;                               
        } 
        Ok(control_time)
    }

    pub fn get_free_treads(&self) -> &VecDeque<usize> {
        &self.free_threads
    }
    
}