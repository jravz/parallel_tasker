# Parallel Task Crate
A fast data parallelism library using Atomics to share data across threads and uniquely pull values from Collections such as Vec, HashMap, Range. It leverages a 'pull' approach and uses a concept of AtomicQueuedValues to reduce the time required for the thread to pick the next value. 
This is achieved by queueing values in advance and allowing each thread to access a unique value within the queue based on an AtomicIsize value, that corresponds to a unique stored value in the Collection. 
Hence, all types that implement the ParallelIter or IntoParallelIter Trait (Vec, HashMap and Range) can be consumed or passed by reference to run Map or ForEach functions on the same.

The results show good performance to the popular Rayon library.

Please try at your end and share your feedback at jayanth.ravindran@gmail.com.

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

