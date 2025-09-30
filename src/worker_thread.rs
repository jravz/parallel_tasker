//! WorkerThreads enables the launching of thread pool that manages the running of closures
//! based on inputs from the Iterator. This follows a pull phiolosophy and thus threads run till
//! there are Items to pull from the AtomicIterator. Whenever a thread becomes free it pulls a new Item
//! and runs the closure function on the same

use std::{mem::ManuallyDrop, pin::Pin, sync::{atomic::AtomicPtr, Arc, Mutex}};

use crate::{collector::Collector, for_each::ParallelForEach, for_each_mut::ParallelForEachMut, iterators::iterator::AtomicIterator, map::ParallelMap, task_queue::TaskQueue};
pub struct WorkerThreads {pub nthreads:usize }

#[allow(dead_code)]
impl WorkerThreads
{

    fn is_queue_active<I,V>(queue:&Arc<AtomicPtr<TaskQueue<I, V>>>) -> bool 
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    V: Send
    {
        if let Some(task_mutex) = unsafe { queue.load(std::sync::atomic::Ordering::Acquire).as_mut()} {
            task_mutex.iter.is_active()
        } else {
            false
        }
    }

    pub fn run<I,F,V>(self, mut task:ParallelForEach<V,F,I>)
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V) + Send + Sync,
    V: Send,    
    {
        let mut vec_handles = Vec::new();   
        let arc_mut_task: Arc<AtomicPtr<TaskQueue<I, V>>> = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(task.iter))));        
        let func = unsafe {*Box::from_raw(Pin::get_unchecked_mut(task.f.as_mut())) };        
        let arc_func = Arc::new(func);
        for _ in 0..self.nthreads {
            let arc_mut_task_clone = arc_mut_task.clone();
            if !Self::is_queue_active(&arc_mut_task_clone) {
                break;
            }
            let builder = std::thread::Builder::new();            
            unsafe {                  
                let arc_func_clone = Arc::clone(&arc_func);                
                let result = builder.spawn_unchecked(move || 
                    {
                        Self::for_each_loop(arc_mut_task_clone,arc_func_clone )
                });   
                vec_handles.push(result);                 
            };                     
        }

        // allow all threads to finish
        for handle in vec_handles {           
            handle.unwrap().join().unwrap();            
        }
        drop(arc_mut_task);

    }

    pub fn run_mut<I,F,V>(self, task:ParallelForEachMut<V,F,I>)
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: FnMut(V) + Send,
    V: Send,    
    {
        let mut vec_handles = Vec::new();   
        let arc_mut_task: Arc<AtomicPtr<TaskQueue<I, V>>> = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(task.iter))));     
        let fclosure = task.f;
        let arc_mut_f = Arc::new(Mutex::new(fclosure));   
        for _ in 0..self.nthreads {
            let arc_mut_task_clone = arc_mut_task.clone();
            if !Self::is_queue_active(&arc_mut_task_clone) {
                break;
            }
            let builder = std::thread::Builder::new();            
            unsafe {                  
                let arc_mut_f_clone =  arc_mut_f.clone();              
                let result = builder.spawn_unchecked(move || 
                    {
                        Self::for_each_mut_loop(arc_mut_task_clone, arc_mut_f_clone)
                });
                vec_handles.push(result);                
            };                     
        }

        // allow all threads to finish
        for handle in vec_handles {           
            handle.unwrap().join().unwrap();            
        }
        drop(arc_mut_task);

    }

    pub fn collect<I,F,T,V,C>(self, mut task:ParallelMap<V,F,T,I>) -> C
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V) -> T + Send + Sync,
    V: Send,
    T:Send,
    C: Collector<T> {
        let mut vec_handles = Vec::new();                        
        let arc_mut_task: Arc<AtomicPtr<TaskQueue<I, V>>> = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(task.iter))));        
        let func = ManuallyDrop::new(unsafe {*Box::from_raw(Pin::get_unchecked_mut(task.f.as_mut())) });
        let arc_func: Arc<ManuallyDrop<F>> = Arc::new(func);
        for _ in 0..(self.nthreads) {
            let arc_mut_task_clone = arc_mut_task.clone();
            
            let builder = std::thread::Builder::new();                  
            let result: Result<std::thread::JoinHandle<Vec<T>>, std::io::Error> = unsafe {                  
                let arc_func_clone = Arc::clone(&arc_func);
                builder.spawn_unchecked(move ||{Self::task_loop(arc_mut_task_clone, arc_func_clone)})                
            };             

            vec_handles.push(result);
        }
        
        let mut output = C::initialize();
        
        for handle in vec_handles { 
            if let Ok(handle_val) = handle {                
                match handle_val.join() {
                    Ok(res) => {
                        output.extend(res.into_iter());
                    }
                    Err(e) => {
                        let e = e.downcast_ref::<String>().unwrap();
                        eprintln!("Error: {}",e);
                    }
                }            
            } else {
                eprintln!("Join Error in Map function of ParallelTask");
            }                                    
        }
        // drop(arc_func);
        // drop(arc_mut_task);
        output
    }        
    
    
    /// Task Loop runs the functions within each spawned thread. The Loop runs till the thread is able 
    /// to pop a value from the Iterator. Once there are no more values from the iterator, the loop breaks and
    /// the thread returns all values obtained till that point
    fn task_loop<I,F,T,V>(task:Arc<AtomicPtr<TaskQueue<I, V>>>, f:Arc<ManuallyDrop<F>>) -> Vec<T> 
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V) -> T + Send,
    V: Send,
    T:Send
    {   
        // let threadid = std::thread::current().id();
        // let mut tot_time = 0;
        // let mut waiting_time = 0;        
        let mut res = Vec::new();  
        // let ttm = std::time::Instant::now();             
        while let Some(input) = {
            // let tm = std::time::Instant::now();
            let val = if let Some(task_mutex) = unsafe { task.load(std::sync::atomic::Ordering::Acquire).as_mut()} 
            {
                task_mutex.pop()
            } else {
                None
            };
            // waiting_time += tm.elapsed().as_micros();            
            val
        } 
        {                                
            let result = f(input);                                    
            res.push(result);                              
        }                
        // tot_time = ttm.elapsed().as_micros();
        // println!("ID: {:?} -> total time = {} micros, waiting time = {} micros",threadid, tot_time, waiting_time);        
        res
    }

    /// Task Loop runs the functions within each spawned thread. The Loop runs till the thread is able 
    /// to pop a value from the Iterator. Once there are no more values from the iterator, the loop breaks and
    /// the thread returns all values obtained till that point
    fn for_each_loop<I,F,V>(task:Arc<AtomicPtr<TaskQueue<I,V>>>, f:Arc<F>)
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: Fn(V),
    V: Send,    
    {   
        // let threadid = std::thread::current().id();
        // let mut tot_time = 0;
        // let mut waiting_time = 0;                  
        // let ttm = std::time::Instant::now();      
        while let Some(input) = {
            // let tm = std::time::Instant::now();
            let val = if let Some(task_mutex) = unsafe { task.load(std::sync::atomic::Ordering::Acquire).as_mut()} 
            {
                task_mutex.pop()
            } else {
                None
            };
            // waiting_time += tm.elapsed().as_micros();
            val
        } 
        {                                           
            f(input);                                                       
        }                
        // tot_time = ttm.elapsed().as_micros();
        // println!("ID: {:?} -> total time = {} micros, waiting time = {} micros",threadid, tot_time, waiting_time);        
    }

    /// For Each Mut Loop runs the functions within each spawned thread. The Loop runs till the thread is able 
    /// to pop a value from the Iterator. Once there are no more values from the iterator, the loop breaks and
    /// the thread returns all values obtained till that point
    /// Considering this uses a Mutex as the the closure is mut, this will be slower than other cases with Fn closures
    fn for_each_mut_loop<I,F,V>(task:Arc<AtomicPtr<TaskQueue<I,V>>>, f:Arc<Mutex<F>>)
    where I:AtomicIterator<AtomicItem = V> + Send + Sized,
    F: FnMut(V),
    V: Send,    
    {   
        // let threadid = std::thread::current().id();
        // let mut tot_time = 0;
        // let mut waiting_time = 0;                  
        // let ttm = std::time::Instant::now();      
        while let Some(input) = {
            // let tm = std::time::Instant::now();
            let val = if let Some(task_mutex) = unsafe { task.load(std::sync::atomic::Ordering::Acquire).as_mut()} 
            {
                task_mutex.pop()
            } else {
                None
            };
            // waiting_time += tm.elapsed().as_micros();
            val
        } 
        {           
            let mut fclosure = f.lock().unwrap();
            fclosure(input);                                                       
        }                
        // tot_time = ttm.elapsed().as_micros();
        // println!("ID: {:?} -> total time = {} micros, waiting time = {} micros",threadid, tot_time, waiting_time);        
    }
}


