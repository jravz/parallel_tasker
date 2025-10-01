# Parallel Task Crate
A fast data parallelism library using Atomics to share data across threads and uniquely pull values from Collections such as Vec, HashMap, Range. It leverages a 'pull' approach and uses a concept of AtomicQueuedValues to reduce the time required for the thread to pick the next value. 

## What is the objective of this crate?
parallel_tasker is a high-performance parallel iteration library for Rust that provides an alternative to Rayon’s work-stealing model. I am a great admirer of Rayon crate and have used it extensively. Instead of making a 'me too', I wanted to experiment with a different algorithm. Unlike Rayon, it does not give each thread its own queue. Instead, it uses a shared atomic counter and a primary + backup batching system:

Atomic counter distribution → threads claim the next available task index directly, guaranteeing unique access without heavy locking.

Primary + backup batches → tasks are grouped into chunks (e.g., 100 items). While threads consume the primary batch, one thread prepares the backup in advance, minimizing wait times when the batch is replenished.

Dynamic task assignment → threads grab the next available task as soon as they finish their current work. This naturally balances workload even when tasks vary in complexity, without needing work stealing.

This design provides low overhead, minimal contention, and predictable cache-friendly access patterns. Benchmarks show that it performs within ~5-10% of Rayon for large workloads, such as Monte Carlo simulations with 100,000 iterations, while maintaining a simpler scheduling logic.

parallel_tasker is a fully usable parallel iterator library for real-world workloads, offering a different perspective on parallelism while maintaining near-optimal performance.

The results show good performance to the popular Rayon library. That said, users are encouraged to test this well within their use cases to ensure suitability and applicability. 

## How this differs from Rayon
parallel_tasker uses a different approach to task scheduling compared to Rayon:

* Shared atomic counter

    * Instead of giving each thread its own deque, all threads pull from a shared atomic index.

    * Each thread gets a unique task without locks, minimizing contention.

* Primary + backup batching

    * Tasks are processed in batches (e.g., 100 items).

    * While threads work through the primary batch, one thread prepares the backup batch ahead of time.

    * This ensures threads never idle waiting for new work.

* Dynamic task assignment

    * Threads grab the next available task as soon as they finish their current one.

    * Heavy tasks are naturally distributed among threads, preventing stragglers without the complexity of work stealing.

Please try at your end and share your feedback at jayanth.ravindran@gmail.com.
Note: if you wish to contribute you are more than welcome.

## Usage example

Add this using:
```
cargo add parallel_task
```

```
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
r1.parallel_iter().for_each(|val| { print!("{} ",*val);});
```

# ShareableAtomicIter:
ShareableAtomicIter enables Vec and HashMap that implement the Fetch trait to easily distributed across threads. Values can be safely accessed without any risk of overlaps. Thus allowing you to design how you wish to process these collections across your threads.
```
use parallel_task::prelude::*;

// Lets create a simple vector of 100 values that we wish to share across 
    // two threads
    let values = (0..100).collect::<Vec<_>>();

    //Lets create a scope to run the two threads in parallel
    std::thread::scope(|s| 
        {
            // use parallel_iter to share by reference and into_parallel_iter to consume the values            
            let parallel_iter = values.parallel_iter();
            // Apply '.shareable()' to get a ShareableAtomicIter within an Arc
            let shared_vec = parallel_iter.shareable();
            //Get a clone for the second thread
            let share_clone = shared_vec.clone();
            
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
```

