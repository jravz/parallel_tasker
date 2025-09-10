use parallel_task::prelude::{ParallelIter,ParallelMapIter};
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
    let r1 = vec_jobs.parallel_iter().map(|func| func()).collect::<Vec<i32>>();    
    println!("Parallel Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();      
    let r2 = vec_jobs.par_iter().map(|v|v()).collect::<Vec<_>>();
    println!("Rayon Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    assert_eq!(r1.len(),r2.len())
}

#[test]
fn comparison_with_rayon_into_case() {    

    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
    let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>();
    
    let tm = std::time::Instant::now();
    let _ = vec_jobs.iter().map(|v| v()).collect::<Vec<_>>();
    println!("INTO CASE --> Non Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());    

    let tm = std::time::Instant::now();    
    let r1 = vec_jobs.clone().into_parallel_iter().map(|func| func()).collect::<Vec<i32>>();    
    println!("INTO CASE --> Parallel Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();      
    let r2 = vec_jobs.into_par_iter().map(|v|v()).collect::<Vec<_>>();
    println!("INTO CASE --> Rayon Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    assert_eq!(r1.len(),r2.len())
}

#[test]
fn length_test() {      
    let res = (0..100_000).collect::<Vec<_>>().parallel_iter().map(|val|*val).collect::<Vec<_>>();
    assert_eq!(res.len(),100_000)
}

#[test]
fn value_test() {      
    let mut res = (0..10_000).collect::<Vec<_>>().parallel_iter().map(|val|*val).collect::<Vec<_>>();
    res.sort();
    let test = (0..10_000).collect::<Vec<i32>>();
    assert_eq!(res,test)
}

#[test]
fn fail_value_test() {      
    let mut res = (0..10_000).collect::<Vec<_>>().parallel_iter().map(|val|val+1).collect::<Vec<i32>>();
    res.sort();
    let test = (0..10_000).collect::<Vec<i32>>();
    assert_ne!(res,test)
}