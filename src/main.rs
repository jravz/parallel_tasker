mod parallel_task;
mod collector;
mod worker_thread;
mod errors;

use parallel_task::ParallelTaskIter;

use rayon::prelude::*;
use tokio::time::Duration;


fn main() {    

    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
    let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>();
    
    let tm = std::time::Instant::now();
    let _ = vec_jobs.iter().map(|v| v()).collect::<Vec<_>>();
    println!("Non Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());    

    let tm = std::time::Instant::now();    
    let _ = vec_jobs.iter().parallel_task(|func| func()).collect::<Vec<i32>>();
    println!("Own Implementation Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();      
    let _ = vec_jobs.par_iter().map(|v|v()).collect::<Vec<_>>();
    println!("Rayon Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

}