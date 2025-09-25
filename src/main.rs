use parallel_task::prelude::*;

fn main() {

    // Test out the AtomicIterator for parallel management of Vectors without risk of overlaps
    let values = (0..100).map(|x|x).collect::<Vec<_>>();

    std::thread::scope(|s| 
        {
            let shared_vec = values.into_parallel_iter().as_arc();
            let share_clone = shared_vec.clone();
            s.spawn(move || {
                let tid = std::thread::current().id();
                while let Some(val) = shared_vec.next(){
                    print!(" [{:?}: {}] ",tid,val);
                }
            });

            s.spawn(move || {
                let tid = std::thread::current().id();
                while let Some(val) = share_clone.next(){
                    print!(" [{:?}: {}] ",tid,val);
                }
            });

        }
    );
    // Map being tested....
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