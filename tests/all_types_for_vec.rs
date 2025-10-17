use parallel_task::prelude::*;


#[test]
fn all_types_test_for_vec() {
    // We will focus only on complex cases that can fail and allow send

    // String
    let v = (0..100_000).map(|x|x.to_string()).collect::<Vec<String>>()
    .into_parallel_iter().map(|x|x).collect::<Vec<_>>();
    assert_eq!(v.len(),100_000);

    // function
    let job = |v:i32| v + 10;
    let v = (0..100_000).map(|_| job.clone() ).collect::<Vec<_>>()
    .into_parallel_iter().map(|x|x(10)).collect::<Vec<_>>();
    assert_eq!(v.len(),100_000);

    // Vec
    let v = (0..100_000).map(|_|vec![1,2,3,4]).collect::<Vec<_>>()
    .into_parallel_iter().map(|mut x|x.push(100)).collect::<Vec<_>>();
    assert_eq!(v.len(),100_000);

}