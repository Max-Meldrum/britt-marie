pub mod hash;
pub mod value;

use crate::data::{Key, Value};

/// Common Index Operations
pub trait IndexOps {
    /// This method ensures all non-persisted data gets pushed to the [storage::RawStore]
    fn persist(&self);
}

/// Operations supported by Ordered Indexes
pub trait OrderedOps: IndexOps {
    /// Fetch value by key
    fn get<K: Key, V: Value>(&self, key: &K) -> Option<&V>;
    /// Blind insert
    fn put<K: Key, V: Value>(&mut self, key: &K, value: V);
    /// Range Scan where returned values are ordered
    fn range<K: Key + Ord, V: Value>(&mut self, start: &K, end: &K) -> Iterator<Item = V>;
}

/// Operations available for a HashIndex
pub trait HashOps<K, V>: IndexOps
where
    K: Key,
    V: Value,
{
    /// Fetch value by key
    fn get(&self, key: &K) -> Option<&V>;
    /// Blind insert
    fn put(&mut self, key: K, value: V);
    /// Read-Modify-Write operation
    fn rmw<F: Sized>(&mut self, key: &K, f: F) -> bool
    where
        F: FnMut(&mut V);
}

/// Operations available for a ValueIndex
pub trait ValueOps<V>: IndexOps
where
    V: Value,
{
    /// Fetch value
    fn get(&self) -> Option<&V>;
    /// Blind insert
    fn put(&mut self, value: V);
    /// Read-Modify-Write operation
    fn rmw<F: Sized>(&mut self, f: F) -> bool
    where
        F: FnMut(&mut V);
}
