# Parallel Task Crate
A super fast data parallelism library with performance comparable or better to Rayon using Atomics to share data across threads and uniquely pull values from Collections such as Vec or HashMap. It follows a 'pull' approach and tries to reduce the time required for the thread to pick the next value.

The tests runs a set of jobs on rayon and this new library. The results show comparable performance and in a number of cases slightly improved performance for this library. No study has been done to establish whether the results are significant.

Please try at your end and share your feedback at jayanth.ravindran@gmail.com.

## Usage example
This crate enables parallel_task to be called on any iter and the result may be collected in to a Vec, HashMap or VecDeque.

use parallel_task::prelude::*;
let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>(); 

// Parallel Iter example
let r1 = vec_jobs.parallel_iter().map(|func| func()).collect::<Vec<i32>>();

// Into Parallel Iter that consumes the vec_jobs
let r1 = vec_jobs.into_parallel_iter().map(|func| func()).collect::<Vec<i32>>();

// Print all values using a for_each. This runs for_each concurrently on a Vec or HashMap


