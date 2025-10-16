use std::time::Duration;

use parallel_task::prelude::*;
use rayon::prelude::*;

fn comparison_with_rayon() {    

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

fn main() {

    // test vis a vis Rayon
    comparison_with_rayon();

    //Map being tested....
    println!("Map Samples");
    //Samples with both parallel_iter and into_parallel_iter options below    
    let mut res = (0..30).collect::<Vec<_>>()
                                                .parallel_iter()
                                                .map(|i| {println!("{}",i);*i})
                                                .collect::<Vec<_>>();    
    res.sort();
    
    println!("res = {:?}",res);
    let mut res2 = res.into_parallel_iter()
    .map(|i| {println!("{}",i);i})
    .collect::<Vec<_>>();  
    res2.sort();    
    
    // For each being tested....
    println!("For Each test");
    res2.parallel_iter()
    .for_each(|val| { print!("{} ",*val);});   

    // testing for Range<i32>
    (0..100).into_parallel_iter()
    .for_each(|v| print!("v:{} ",v));

}