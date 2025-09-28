use parallel_task::{map::ParallelMapIter, prelude::IntoParallelIter};
use rayon::prelude::*;

#[test]
fn simple_test() {

    let tm = std::time::Instant::now();
    let vec = (0..1000000).map(|v|v+100i32).collect::<Vec<_>>();   
    println!("Non parallel: {}",tm.elapsed().as_nanos());

    let tm = std::time::Instant::now();
    let vec = (0..1_000_000).into_parallel_iter().map(|v|v+100i32).threads(3).collect::<Vec<_>>();  
    println!("PT: {}",tm.elapsed().as_nanos());  

    let tm = std::time::Instant::now();
    let vec = (0..1_000_000).into_par_iter().map(|v|v+100i32).collect::<Vec<_>>();  
    println!("Rayon: {}",tm.elapsed().as_nanos());  

    assert_eq!(1,1);
}
