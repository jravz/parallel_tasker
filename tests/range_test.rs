use parallel_task::{map::ParallelMapIter, prelude::IntoParallelIter};


#[test]
fn range_test() {
    let vec = (0..10000).into_parallel_iter().map(|v|v).collect::<Vec<_>>();    

    assert_eq!(vec.len(),10000);
}
