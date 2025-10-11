use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use std::time::{Duration, Instant};
use rayon::prelude::*;
use parallel_task::prelude::*;
use rand::Rng;

#[derive(Debug, Clone)]
struct Job {
    id: usize,
    duration: Duration,
}

impl Job {
    fn run(&self) {
        let start = Instant::now();
        while start.elapsed() < self.duration {
            std::hint::spin_loop();
        }
    }
}

fn create_micro_jobs(count: usize) -> Vec<Job> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|id| {
            let p = rng.gen_range(0..100);
            let micros = if p < 90 {
                rng.gen_range(50..=500)
            } else if p < 99 {
                rng.gen_range(500..=3000)
            } else {
                rng.gen_range(3000..=10000)
            };
            Job {
                id,
                duration: Duration::from_micros(micros),
            }
        })
        .collect()
}

fn bench_micro_jobs(c: &mut Criterion) {
    let job_count = 10_000;
    let base_jobs = create_micro_jobs(job_count);

    // -------------------
    // OWNED iterators
    // -------------------
    c.bench_function("Normal (owned)", |b| {
        b.iter_batched(
            || base_jobs.clone(), // setup (not timed)
            |jobs| {
                jobs.into_iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("Rayon (owned)", |b| {
        b.iter_batched(
            || base_jobs.clone(),
            |jobs| {
                jobs.into_par_iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("PT (owned)", |b| {
        b.iter_batched(
            || base_jobs.clone(),
            |jobs| {
                jobs.into_parallel_iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });

    // -------------------
    // BORROWED iterators
    // -------------------
    c.bench_function("Normal (&iter)", |b| {
        b.iter_batched_ref(
            || base_jobs.clone(),
            |jobs| {
                jobs.iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("Rayon (&iter)", |b| {
        b.iter_batched_ref(
            || base_jobs.clone(),
            |jobs| {
                jobs.par_iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("PT (&iter)", |b| {
        b.iter_batched_ref(
            || base_jobs.clone(),
            |jobs| {
                jobs.parallel_iter().for_each(|j| j.run());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_micro_jobs);
criterion_main!(benches);
