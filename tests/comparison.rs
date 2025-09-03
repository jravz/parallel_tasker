use parallel_task::prelude::*;
use rayon::prelude::*;
use tokio::time::Duration;


// Run 100,000 jobs across three use cases:
// 1. Sequential using a normal iter with single thread
// 2. Run with ParallelTaskIter
// 3. Run with Rayon
// ParallelTask does a commendable job matching Rayon in performance
#[test]
fn comparison_with_rayon() {    

    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
    let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>();
    
    let tm = std::time::Instant::now();
    let _ = vec_jobs.iter().map(|v| v()).collect::<Vec<_>>();
    println!("Non Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());    

    let tm = std::time::Instant::now();    
    let r1 = vec_jobs.iter().parallel_task(|func| func()).collect::<Vec<i32>>();
    println!("Parallel Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();      
    let r2 = vec_jobs.par_iter().map(|v|v()).collect::<Vec<_>>();
    println!("Rayon Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    assert_eq!(r1.len(),r2.len())
}