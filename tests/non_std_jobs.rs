use std::time::{Duration, Instant};
use std::thread;
use rand::Rng;
use parallel_task::prelude::*;
use rayon::prelude::*;

#[derive(Debug,Clone)]
struct Job {
    runtime_us: u64, // microseconds
}

fn generate_nonstandard_jobs() -> Vec<Job> {
    let mut rng = rand::rng();

    // 100 jobs, each with 1 µs – 50 ms runtime
    let mut jobs: Vec<Job> = (0..100)
        .map(|_| {
            let runtime_us = rng.random_range(500..=50_000); // 0.5 ms to 50 ms
            Job {runtime_us }
        })
        .collect();

    // Normalize to roughly 1 second total runtime
    let total_us: u64 = jobs.iter().map(|j| j.runtime_us).sum();
    let scale = 1_000_000f64 / total_us as f64; // target ≈ 1_000_000 µs = 1 s
    for j in &mut jobs {
        j.runtime_us = (j.runtime_us as f64 * scale) as u64;
    }

    jobs
}

#[test]
fn non_std_jobs() {

    println!("Into iters");
    let vec_jobs = generate_nonstandard_jobs();
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    _ = vec_jobs.into_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());
 
    let tm = std::time::Instant::now();
    vec_jobs1.into_par_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.into_parallel_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    println!("& iters");
    let vec_jobs = generate_nonstandard_jobs();
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    _ = vec_jobs.into_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs1.par_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.parallel_iter().map(|job|{thread::sleep(Duration::from_micros(job.runtime_us));})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    assert_eq!(1,1);
}
