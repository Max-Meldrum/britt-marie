pub mod hash;

use crate::data::{Key, Value};

/// Operations supported by Ordered Indexes
pub trait OrderedOps {
    /// Fetch value by key
    fn get<K: Key, V: Value>(&self, key: &K) -> Option<&V>;
    /// Blind insert
    fn put<K: Key, V: Value>(&mut self, key: &K, value: V);
    /// Range Scan where returned values are ordered
    fn range<K: Key + Ord, V: Value>(&mut self, start: &K, end: &K) -> Iterator<Item = V>;
}

/// Operations supported by Random Indexes
pub trait RandomOps<K, V>
where
    K: Key,
    V: Value,
{
    /// Fetch value by key
    fn get(&self, key: &K) -> Option<&V>;
    /// Blind insert
    fn put(&mut self, key: &K, value: V);
    /// Read-Modify-Write operation
    fn rmw<F: Sized>(&mut self, key: &K, f: F)
    where
        F: FnMut(&mut V);
}
