use parallel_task::{map::ParallelMapIter, prelude::IntoParallelIter};


#[test]
fn string_test() {
    let vec = (0..100).map(|v|v.to_string()).collect::<Vec<_>>();

    let vec2 = vec.clone().into_parallel_iter().map(|v|v).collect::<Vec<_>>();

    assert_eq!(vec.len(),vec2.len());
}
