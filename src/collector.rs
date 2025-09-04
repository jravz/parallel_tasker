//! Collector is a trait that can be implemented across Collections and other types that implement
//! Extend trait. For instance here it has been implemented for Vector, HashMap and so on. This allows
//! the end result to collected in the desired form as per the annotation.

use std::hash::Hash;
use std::collections::{HashMap, VecDeque};

pub trait Collector<T>:Extend<T> {
    fn initialize() -> Self;
}

impl<T> Collector<T> for Vec<T> {
    fn initialize() -> Self {
        Vec::new()
    }
}

impl<K,V> Collector<(K,V)> for HashMap<K,V>
where K: Eq + Hash + Send,
    V: Send,
{
    fn initialize() -> Self {
        let hsh:HashMap<K,V> = HashMap::new();
        hsh
    }
}

impl<T> Collector<T> for VecDeque<T> {
    fn initialize() -> Self {
        VecDeque::new()
    }
}

