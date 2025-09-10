use std::collections::HashMap;

pub trait Fetch {
    type FetchedItem;
    type FetchKey;
    type FetchRefItem<'a>
    where
        Self: 'a;

    fn keys_vec(&self) -> Vec<Self::FetchKey>;
    fn get_key<'b>(keys: &'b [Self::FetchKey], pos: &'b usize) -> Option<&'b Self::FetchKey>;
    fn atomic_get<'a>(&'a self, index: &Self::FetchKey) -> Option<Self::FetchRefItem<'a>>;
    fn atomic_fetch(&mut self, index: &Self::FetchKey) -> Option<Self::FetchedItem>;
}

impl<T> Fetch for Vec<T>
{        
    type FetchedItem = T;        
    type FetchKey = usize;
    type FetchRefItem<'a> = &'a Self::FetchedItem
    where Self: 'a;

    fn keys_vec(&self) ->  Vec<Self::FetchKey> {
        Vec::new()
    }

    fn get_key<'a>(_:&[usize], pos:&'a usize) -> Option<&'a Self::FetchKey> {
        Some(pos)
    } 

    fn atomic_get(&self, index:&usize) -> Option<&Self::FetchedItem> {
        self.get(*index)
    }

    fn atomic_fetch(&mut self,index:&usize) -> Option<Self::FetchedItem> {
        // Lets first put a safeguard to ensure the wrong value does not get returned
        if *index >= self.len() { return None; }
        
        //This is the one logical way to return the actual value within the Vector
        //I am open to better suggestions here
        let mut value: T = unsafe { std::mem::zeroed() };
        std::mem::swap(&mut value, &mut self[*index]);
        Some(value)
    }       
}



impl<K, V> Fetch for HashMap<K, V>
where    
    K: std::cmp::Eq + std::hash::Hash + Clone,
{
    type FetchedItem = (K, V);
    type FetchKey = K;
    type FetchRefItem<'a> = (&'a K, &'a V)
    where
        Self: 'a;

    fn keys_vec(&self) -> Vec<Self::FetchKey> {
        self.keys().cloned().collect::<Vec<K>>()
    }

    fn get_key<'b>(keys: &'b [K], pos: &'b usize) -> Option<&'b Self::FetchKey> {
        keys.get(*pos)
    }

    fn atomic_get<'a>(&'a self, index: &K) -> Option<Self::FetchRefItem<'a>> {
        self.get_key_value(index)
    }

    fn atomic_fetch(&mut self, index: &K) -> Option<Self::FetchedItem> {
        self.remove_entry(index)
    }
}
