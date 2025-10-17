use parallel_task::prelude::{ParallelIter, ParallelMapIter};


#[test]
fn length_test() {      
    let res = (0..100_000).collect::<Vec<_>>().parallel_iter().map(|val|*val).collect::<Vec<_>>();
    assert_eq!(res.len(),100_000)
}

#[test]
fn value_test() {      
    let mut res = (0..10_000).collect::<Vec<_>>().parallel_iter().map(|val|*val).collect::<Vec<_>>();
    res.sort();
    let test = (0..10_000).collect::<Vec<i32>>();
    assert_eq!(res,test)
}

#[test]
fn fail_value_test() {      
    let mut res = (0..10_000).collect::<Vec<_>>().parallel_iter().map(|val|val+1).collect::<Vec<i32>>();
    res.sort();
    let test = (0..10_000).collect::<Vec<i32>>();
    assert_ne!(res,test)
}