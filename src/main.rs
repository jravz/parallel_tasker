use parallel_task::prelude::{ParallelIter,ParallelTaskIter};

fn main() {    
    let mut res = (0..30).collect::<Vec<_>>()
                                                .parallel_iter()
                                                .parallel_task(|i| {println!("{}",i);*i})
                                                .collect::<Vec<_>>();    
    res.sort();
    println!("res = {:?}", res);
    
}