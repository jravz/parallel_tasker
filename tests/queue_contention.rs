use parallel_task::{
accessors::limit_queue::LimitAccessQueue,
push_workers::worker_thread::Coordination};

/// Tests for queue contention between primary and secondary accessors
#[test]
fn queue_contention() {
    let values = (0..100_000).collect::<Vec<_>>();

    let (mut primary, secondary) = LimitAccessQueue::<i32,Coordination>::new();
    _ = primary.write(values);

    std::thread::scope(move |s| {
        let handle1 = s.spawn(move ||
            {           
                let mut res = Vec::new();
                for _ in 0..40_000 {                                         
                    if let Some(value ) = primary.pop() {                                                
                        res.push(value);
                    } else {
                        break;
                    }                                        
                }

                if let Some(values )= primary.steal() {
                    res.extend(values.into_iter());
                }  
                res              
            }
        ); 


        let handle2 = s.spawn(
            move || {
                let mut res = Vec::new();
                for _ in 0..40_000 {                                                        
                    if let Some(values ) = secondary.pop() {                                                            
                        res.push(values);
                    } else {
                        break;
                    }
                    
                }
                res
            }
        );    

        let results1 = handle1.join().unwrap();
        let results2 = handle2.join().unwrap();

        // assert that secondary at least got some values and not just primary. Allowing two queues act in parallel
        // 100,000 is a large enough number to give enough opportunities for both accessors
        assert!(!results1.is_empty());
        assert!(!results2.is_empty());

        // assert that sum of both the vectors is 1000
        assert_eq!(results1.len() + results2.len(), 100_000);

    });

}