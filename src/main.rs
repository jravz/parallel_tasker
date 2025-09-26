use parallel_task::prelude::*;
use std::sync::Arc;
use parallel_task::iterators::prelude::{ShareableAtomicIter, ParallelIter, ParallelIterator,AtomicIterator};

fn main() {

    // Test out the AtomicIterator for parallel management of Vectors without risk of overlaps
    
    // Lets create a simple vector of 100 values that we wish to share across 
    // two threads
    let values = (0..100).collect::<Vec<_>>();

    //Lets create a scope to run the two threads in parallel
    std::thread::scope(|s| 
        {
            // use parallel_iter to share by reference and into_parallel_iter to consume the values            
            let parallel_iter: parallel_task::iterators::prelude::ParallelIterator<'_, Vec<i32>> = values.parallel_iter();
            // Apply '.shareable()' to get a ShareableAtomicIter within an Arc
            let shared_vec: Arc<ShareableAtomicIter<ParallelIterator<'_, Vec<i32>>>> = parallel_iter.shareable();
            //Get a clone for the second thread
            let share_clone: Arc<ShareableAtomicIter<ParallelIterator<'_, Vec<i32>>>> = shared_vec.clone();
            
            // Lets spawn the first thread
            s.spawn(move || {
                let tid = std::thread::current().id();
                //Just do .next() and get unique values without overlap with other threads
                while let Some(val) = shared_vec.next(){
                    print!(" [{:?}: {}] ",tid,val);
                }
            });

            // Lets spawn the second thread
            s.spawn(move || {
                let tid = std::thread::current().id();
                while let Some(val) = share_clone.next(){
                    print!(" [{:?}: {}] ",tid,val);
                }
            });

        }
    );
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

    //testing mutability of for_each_mut
    println!("For Each Mut test");
    let mut test = 0;
    let target = 100;
    (0..=target).collect::<Vec<i32>>().
    parallel_iter().for_each_mut(|v| { test += v;});
    println!("X = {}, Expected = {}",test, (target * (target + 1)/2));

}