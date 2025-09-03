use std::hash::Hash;
use std::collections::HashMap;

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