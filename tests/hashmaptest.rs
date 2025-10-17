use std::collections::HashMap;
use std::time::Duration;
use parallel_task::prelude::{ParallelIter,ParallelMapIter,IntoParallelIter};

#[test]
fn hashmap_test() {   
    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };

    let hashmap_jobs = (0..100_00).map(|i|(i,job)).collect::<HashMap<_,_>>();

    let h1 = hashmap_jobs.parallel_iter().
    map(|(_,job)|job())
    .collect::<Vec<_>>();    

    let hashtemp = hashmap_jobs.clone();    
    let h2 = hashtemp.into_parallel_iter().
    map(|(_,job)|job())
    .collect::<Vec<_>>();    


    assert_eq!(h1.len(),h2.len());
}