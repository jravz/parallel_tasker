//!Gives flexibility to the algorithm to select from a set of available prioritization approaches.
//! The threads are prioritized every time the master thread polls each individual thread. 
//! When polling, a vector prioritization is created as a vector. The threads are approached in sequence
//! to pull away half the values in the queue. 
//! The first thread in the vector then is either a laggard or has the maximum tasks in the queue or other parameter.

use std::cmp::Ordering;
use super::thread_manager::ThreadManager;

/// Independent approaches to thread prioritization may be implemented using PrioritizeThread trait.
/// This is an input parameter when defining the Worker Controller.
/// It involves implementing the prioritize algorithm that returns a vector having a tuple of 4 values.
/// Values in sequence are:
/// a. threadpos - usize (used to pick the thread from the thread_manager)
/// b. remaining values in the queue - usize (available from WorkerThread)
pub trait PrioritizeThread {
    fn prioritize<'env, 'scope,Input,Output,F>(&self, thread_manager:&mut ThreadManager<'env, 'scope,Input,Output,F>) 
    -> Vec<(usize, usize)>
    where Input: Send + Sync + 'scope,
    Output: Send + Sync + 'scope,  
    F: Fn(Input) -> Output + Send + Sync + 'scope,
    'env: 'scope;   
}

#[allow(dead_code)]
/// ThreadPrioritization is the default approach passed to Worker Controller, used for prioritization of threads within the library.
pub enum ThreadPrioritization {
    Remaining,
    RateOfChange,
    ChangeFromLastPoll
}

impl PrioritizeThread for ThreadPrioritization {

    fn prioritize<'env, 'scope,Input,Output,F>(&self, thread_manager:&mut ThreadManager<'env, 'scope,Input,Output,F>) 
    -> Vec<(usize, usize)>
    where Input: Send + Sync + 'scope,
    Output: Send + Sync + 'scope,  
    F: Fn(Input) -> Output + Send + Sync + 'scope,
    'env: 'scope
    {
        let mut vec_ranking = thread_manager.threads_as_mutable()
        .iter_mut().filter_map(|thread|
        {
            thread.poll_progress().map(|stats|
                {
                    (thread.pos(),stats.0, stats.1, stats.2)
                }
            )
        }).collect::<Vec<(usize, usize, usize, f64)>>();
        
        match self {
            ThreadPrioritization::ChangeFromLastPoll => {
                vec_ranking.sort_by(|a,b|{
                    if a.2 > b.2 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                });                
            }
            ThreadPrioritization::RateOfChange => {
                vec_ranking.sort_by(|a,b|{
                    if a.3 > b.3 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                });                
            }
            ThreadPrioritization::Remaining => {
                vec_ranking.sort_by(|a,b|{
                    if a.1 < b.1 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                });
            }
        }        
        vec_ranking.into_iter().map(|val|(val.0,val.1))
        .collect::<Vec<_>>()
    }

}