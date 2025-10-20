use parallel_task::prelude::*;
use rayon::prelude::*;


#[test]
fn simple_test() {
       
    let vec = (0..1_000_000).into_iter().collect::<Vec<_>>();
    let tm = std::time::Instant::now();
    println!("PT vec: {}",tm.elapsed().as_micros()); 
    let iter = vec.into_parallel_iter();
    println!("PT iter: {}",tm.elapsed().as_micros()); 
    let map = iter.map(|v|v+100i32);
    println!("PT map: {}",tm.elapsed().as_micros()); 
    let coll = map.collect::<Vec<_>>();  
    println!("PT collect: {}",tm.elapsed().as_micros());  

    let vec = (0..1_000_000).into_iter().collect::<Vec<_>>();
    let tm = std::time::Instant::now();
    vec.into_par_iter().map(|v|v+100i32).collect::<Vec<_>>();  
    println!("Rayon: {}",tm.elapsed().as_micros());  

    assert_eq!(coll.len(),1_000_000);
}

