# Parallel Task Crate
A super fast data parallelism library using Atomics to share data across threads and uniquely pull values from Collections such as Vec or HashMap. It follows a 'pull' approach and tries to reduce the time required for the thread to pick the next value. The key question was can the time taken by thread to get the next job be kept minimal.
This is achieved by using an AtomicIterator that generates a mutually exclusive usize value for each thread that corresponds to a unique stored value in the Collection. 
Hence, all types that implement the Fetch Trait (Vec and HashMap) can be consumed or passed by reference to run Map or ForEach functions on the same.

The results show comparable performance to the popular Rayon library and in a number of cases improved performance as well. No study has been done to establish whether the results are significant.

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
```

