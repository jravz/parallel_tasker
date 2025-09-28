use parallel_task::{map::ParallelMapIter, prelude::IntoParallelIter};


#[test]
fn string_test() {
    //This test has been setup since the original algorithms panicked when doing parallel processing
    // on Vec<String> and other types which do not have a fixed size
    let vec = (0..100).map(|v|v.to_string()).collect::<Vec<_>>();

    let vec2 = vec.clone().into_parallel_iter().map(|v|v).collect::<Vec<_>>();

    assert_eq!(vec.len(),vec2.len());
}
