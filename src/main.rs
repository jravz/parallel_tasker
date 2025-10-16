use std::{collections::HashMap, time::Duration};

use parallel_task::prelude::*;
use rand::Rng;
use rayon::prelude::*;

#[derive(Debug,Clone)]
struct Job {
    runtime_us: u64, // microseconds
}

fn generate_nonstandard_jobs() -> Vec<Job> {
    let mut rng = rand::rng();

    // 100 jobs, each with 1 µs – 50 ms runtime
    let mut jobs: Vec<Job> = (0..100)
        .map(|i| {
            let runtime_us = rng.random_range(500..=50_000); // 0.5 ms to 50 ms
            Job {runtime_us }
        })
        .collect();

    // Normalize to roughly 1 second total runtime
    let total_us: u64 = jobs.iter().map(|j| j.runtime_us).sum();
    let scale = 1_000_000f64 / total_us as f64; // target ≈ 1_000_000 µs = 1 s
    for j in &mut jobs {
        j.runtime_us = (j.runtime_us as f64 * scale) as u64;
    }

    jobs
}

fn non_std_jobs() {
    println!("NON STD JOBS");
    println!("Into iters");
    let vec_jobs = generate_nonstandard_jobs();
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    vec_jobs.into_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());
 
    let tm = std::time::Instant::now();
    vec_jobs1.into_par_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.into_parallel_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    println!("& iters");
    let vec_jobs = generate_nonstandard_jobs();
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    vec_jobs.into_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs1.par_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.parallel_iter().map(|job|{std::thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    assert_eq!(1,1);
}


fn simple_test() {

    println!("RUNNING SIMPLE TEST COMPARISON");
    //vec level testing
    let tm = std::time::Instant::now();
    let vec = (0..1_000_000).map(|v|v+100i32).collect::<Vec<_>>();   
    println!("Non parallel: {}",tm.elapsed().as_micros());

    
    let vec = (0..1_000_000).into_iter().collect::<Vec<_>>();
    let tm = std::time::Instant::now();
    println!("PT vec: {}",tm.elapsed().as_micros()); 
    let iter = vec.into_parallel_iter();
    println!("PT iter: {}",tm.elapsed().as_micros()); 
    let map = iter.map(|v|v+100i32);
    println!("PT map: {}",tm.elapsed().as_micros()); 
    let coll = map.collect::<Vec<_>>();  
    println!("PT collect: {}",tm.elapsed().as_micros());  

    let tm = std::time::Instant::now();
    let vec = (0..1_000_000).into_par_iter();
    println!("Rayon iter: {}",tm.elapsed().as_micros()); 
    let map = vec.map(|v|v+100i32);
    println!("Rayon map: {}",tm.elapsed().as_micros()); 
    let coll = map.collect::<Vec<_>>();  
    println!("Rayon collect: {}",tm.elapsed().as_micros()); 

    assert_eq!(1,1);
}



fn comparison_with_rayon() {    
    println!("RUNNING COMPARISON WITH RAYON");
    let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
    let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>();
    
    let tm = std::time::Instant::now();
    let _ = vec_jobs.iter().map(|v| v()).collect::<Vec<_>>();
    println!("Non Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());    

    let v = vec_jobs.clone();
    let tm = std::time::Instant::now();    
    let r2 = v.into_par_iter().map(|v|v()).collect::<Vec<_>>();
    println!("Rayon Task Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    let tm = std::time::Instant::now();          
    let r1 = vec_jobs.into_parallel_iter().map(|func| func()).collect::<Vec<i32>>();    
    println!("PT Parallel Time elapsed: {} microseconds.",tm.elapsed().as_micros());       

    assert_eq!(r1.len(),r2.len())
}

fn hashmap_test() {   
    println!("HashMap test");

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

fn main() {
    simple_test();
    // // test vis a vis Rayon
    comparison_with_rayon();

    non_std_jobs();

    hashmap_test();

    // //Map being tested....
    // println!("Map Samples");
    // //Samples with both parallel_iter and into_parallel_iter options below    
    // let mut res = (0..30).collect::<Vec<_>>()
    //                                             .parallel_iter()
    //                                             .map(|i| {println!("{}",i);*i})
    //                                             .collect::<Vec<_>>();    
    // res.sort();
    
    // println!("res = {:?}",res);
    // let mut res2 = res.into_parallel_iter()
    // .map(|i| {println!("{}",i);i})
    // .collect::<Vec<_>>();  
    // res2.sort();    
    
    // // For each being tested....
    // println!("For Each test");
    // res2.parallel_iter()
    // .for_each(|val| { print!("{} ",*val);});   

    // // testing for Range<i32>
    // (0..100).into_parallel_iter()
    // .for_each(|v| print!("v:{} ",v));

}