use std::collections::HashMap;
use rayon::prelude::*;
use parallel_task::prelude::*;

fn main() {    
    let mut res: Vec<i32> = (0..30).collect::<Vec<_>>().parallel_task(|i| {println!("{}",i);*i}).threads(3).collect::<Vec<_>>();
    res.sort();
    // println!("res = {:?}", res);
    // let x = res.iter();
    
    let y = res.drain(0..1).nex;

    let mut hsh = HashMap::new();
    hsh.insert(1, "Pony".to_owned());
    hsh.insert(12 ,"Tony".to_owned());
    
    let y = hsh.iter().next();
    let rng = (0..12);
    let val = rng[1];
    let val = hsh.get_key_value(k);
    let (y,x) = val;
    


    let j = HashMap::new();
    
}