use std::collections::HashMap;
use std::time::Duration;
use rayon::prelude::*;
use parallel_task::prelude::{ParallelIter,ParallelMapIter,IntoParallelIter};

#[test]
fn hashmap_test() {   
    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };

    let hashmap_jobs = (0..100_00).map(|i|(i,job)).collect::<HashMap<_,_>>();

    let tm = std::time::Instant::now();
    let h1 = hashmap_jobs.parallel_iter().
    map(|(i,job)|job())
    .collect::<Vec<_>>();
    println!("PT - HashMap Iter Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();
    let h2 = hashmap_jobs.par_iter().
    map(|(i,job)|job())
    .collect::<Vec<_>>();
    println!("Rayon - HashMap Iter Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());     

    assert_eq!(h1.len(),h2.len());

    let hashtemp = hashmap_jobs.clone();
    let tm = std::time::Instant::now();
    let h1 = hashtemp.into_parallel_iter().
    map(|(i,job)|job())
    .collect::<Vec<_>>();
    println!("PT - HashMap IntoIter Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();
    let h2 = hashmap_jobs.into_parallel_iter().
    map(|(i,job)|job())
    .collect::<Vec<_>>();
    println!("Rayon - HashMap IntoIter Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       
    assert_eq!(h1.len(),h2.len());
}