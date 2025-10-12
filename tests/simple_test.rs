use std::{collections::HashMap, hash::Hash};

use parallel_task::prelude::*;
use rayon::prelude::*;

#[test]
fn simple_test() {


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


    // //range tests
    // let tm = std::time::Instant::now();
    // let hsh1 = (0..1_000_000).into_parallel_iter()
    // .map(|x|(x,x+100)).collect::<HashMap<i32,i32>>();
    // println!("PT range: {}",tm.elapsed().as_micros()); 

    // let tm = std::time::Instant::now();
    // let hsh2 = (0..1_000_000).into_par_iter()
    // .map(|x|(x,x+100)).collect::<HashMap<i32,i32>>();
    // println!("Rayon range: {}",tm.elapsed().as_micros()); 

    // //hashmap tests
    // let hsh_norm = hsh1.clone();
    // let tm = std::time::Instant::now();
    // let _ = hsh_norm.into_iter().map(|(k,v)| (k,v+10)).collect::<HashMap<i32,i32>>();
    // println!("Std hashmap: {}",tm.elapsed().as_micros()); 

    // let tm = std::time::Instant::now();
    // let _ = hsh1.into_parallel_iter().map(|(k,v)| (k,v+10)).collect::<HashMap<i32,i32>>();
    // println!("PT hashmap: {}",tm.elapsed().as_micros()); 

    // let tm = std::time::Instant::now();
    // let _ = hsh2.into_par_iter().map(|(k,v)| (k,v+10)).collect::<HashMap<i32,i32>>();
    // println!("Rayon hashmap: {}",tm.elapsed().as_micros()); 

    assert_eq!(1,1);
}

