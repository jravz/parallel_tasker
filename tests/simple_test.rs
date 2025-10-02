use parallel_task::prelude::*;
use rayon::prelude::*;

#[test]
fn simple_test() {

    let tm = std::time::Instant::now();
    let vec = (0..1_000_000).map(|v|v+100i32).collect::<Vec<_>>();   
    println!("Non parallel: {}",tm.elapsed().as_micros());

    
    let vec = (0..1_000_000).into_iter().collect::<Vec<_>>();
    let tm = std::time::Instant::now();
    println!("PT vec: {}",tm.elapsed().as_micros()); 
    let iter = vec.into_parallel_iter();
    println!("PT iter: {}",tm.elapsed().as_micros()); 
    let map = iter.map(|v|v+100i32).threads(8);
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

