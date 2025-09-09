use parallel_task::prelude::*;

fn main() {    
    let mut res = (0..30).collect::<Vec<_>>().parallel_task(|i| {println!("{}",i);*i}).threads(3).collect::<Vec<_>>();
    res.sort();
    println!("res = {:?}", res);
}