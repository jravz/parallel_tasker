use std::{time::{Duration, Instant}};
use rayon::prelude::*;
use parallel_task::prelude::*;
use rand::Rng;

#[derive(Debug, Clone)]
struct Job {
    duration: Duration,
}

impl Job {
    fn run(&self) {
        // simulate real CPU work proportional to duration
        let start = Instant::now();
        while start.elapsed() < self.duration {
            std::hint::spin_loop();
        }
    }
}

fn create_micro_jobs(count: usize) -> Vec<Job> {
    let mut rng = rand::rng();

    (0..count)
        .map(|_id| {
            // 90% jobs: ultra fast (50–500 µs)
            // 9% jobs: moderate (500–3000 µs)
            // 1% jobs: heavier (3–10 ms)
            let p = rng.random_range(0..100);
            let micros = if p < 90 {
                rng.random_range(50..=500)
            } else if p < 99 {
                rng.random_range(500..=3000)
            } else {
                rng.random_range(3000..=10000)
            };
            Job {                
                duration: Duration::from_micros(micros),
            }
        })
        .collect()
}


#[test]
fn micro_jobs() {

    println!("Into iters");
    let vec_jobs = create_micro_jobs(10_000);
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    _ = vec_jobs.into_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());
 
    let tm = std::time::Instant::now();
    vec_jobs1.into_par_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.into_parallel_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    println!("& iters");
    let vec_jobs = create_micro_jobs(10_000);
    let vec_jobs1 = vec_jobs.clone();
    let vec_jobs2 = vec_jobs.clone();
    let tm = std::time::Instant::now();
    _ = vec_jobs.into_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();
    println!("Normal : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs1.par_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();  
    println!("Rayon : {}",tm.elapsed().as_micros());

    let tm = std::time::Instant::now();
    vec_jobs2.parallel_iter().map(|job|{job.run();})
    .collect::<Vec<_>>();
    println!("PT : {}",tm.elapsed().as_micros());

    assert_eq!(1,1);
}
