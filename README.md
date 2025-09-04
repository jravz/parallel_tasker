# parallel_tasker
Build a data parallelism library similar to Rayon with comparable performance as a simpler replacement for most use cases.

ParallelTaskIter is an experiment to create a simple module to help manage CPU intensive jobs across threads. This proposes that a work stealing algorithm is not always necessary and a simple pull (.next) based approach can be equally effective in specific use case.

The main.rs runs a set of jobs on rayon and this new library. The results show comparable performance and in majority cases slightly improved performance for this library. The reason could be a lower overhead. This does not state that Rayon can be replaced. But the objective is to allow general users to understand the functioning of a data parallelism library and also to allow them to opt for their own options when needed.

## Usage example
This crate enables parallel_task to be called on any iter and the result may be collected in to a Vec, HashMap or VecDeque.
```
use parallel_task::prelude::*;
let job = || {              
        std::thread::sleep(Duration::from_nanos(10)); 
        (0..1_000).sum::<i32>()
    };
let vec_jobs = (0..100_000).map(|_|job).collect::<Vec<_>>(); 

let r1 = vec_jobs.iter().parallel_task(|func| func()).collect::<Vec<i32>>();
```
