use parallel_task::prelude::{ParallelIter,ParallelMapIter};
use sysinfo::System;
use rand::Rng;
use rayon::prelude::*;

//Run multiple consecutive tests to check how well the library holds up

fn print_process_memory(message:&str) {
    let mut sys = System::new_all();
    sys.refresh_all();

    let pid = sysinfo::get_current_pid().unwrap();
    let process = sys.process(pid).unwrap();

    println!("{}: {} KB", message, process.memory());
}

fn job1() -> i64 {
    let randval = rand::rng().random_range(200_000..400_000);
    (0..randval).sum()
}

fn job2() -> u128 {
    let randval = rand::rng().random_range(200_000..400_000);
    (0..randval).fold(1,|acc,val| acc + val)
}

#[test]
fn consecutive_heavy_process() {
    println!("Starting Rayon");

    let t1 = std::time::Instant::now();
    let jobs = (0..5_000).map(|_|job1).collect::<Vec<_>>();

    print_process_memory("Before job1");
    let _ = jobs.par_iter().map(|j|j()).collect::<Vec<i64>>();

    print_process_memory("Post job1");

    let jobs = (0..5_000).map(|_|job2).collect::<Vec<_>>();
    
    print_process_memory("Before job2");
    let _ = jobs.par_iter().map(|j|j()).collect::<Vec<u128>>();
    print_process_memory("Post job2");

    println!("Rayon Task implementation - total time - {} milliseconds.",t1.elapsed().as_millis());
    println!("Starting Parallel Task");

    let t1 = std::time::Instant::now();
    let jobs = (0..5_000).map(|_|job1).collect::<Vec<_>>();

    print_process_memory("Before job1");
    let _ = jobs.parallel_iter().map(|j|j()).collect::<Vec<i64>>();

    print_process_memory("Post job1");

    let jobs = (0..5_000).map(|_|job2).collect::<Vec<_>>();
    
    print_process_memory("Before job2");
    let _ = jobs.parallel_iter().map(|j|j()).collect::<Vec<u128>>();
    print_process_memory("Post job2");

    println!("Parallel Task implementation - total time - {} milliseconds.",t1.elapsed().as_millis());

    assert_eq!(1,1)
}