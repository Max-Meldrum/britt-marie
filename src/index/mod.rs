pub mod hash;
// TODO
//pub mod art;

use crate::data::{Entry, Key, LazyEntry, RawEntry, Value};
use hash::HashTable;
use std::hash::Hash;
use std::mem::{self, MaybeUninit};
use std::ptr::NonNull;

pub enum Index<K, V>
where
    K: Key,
    V: Value,
{
    Hash(HashTable<K, V>),
    Value(Entry<V>),
}

impl<K, V> Index<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    #[inline]
    pub(crate) fn get(&self, key: &K) -> Option<&Entry<V>> {
        match &*self {
            Index::Hash(table) => table.get(key),
            Index::Value(ref entry) => Some(entry),
        }
    }
    #[inline]
    pub(crate) fn put(&mut self, key: K, entry: Entry<V>) -> Option<Entry<V>> {
        match *self {
            Index::Hash(ref mut table) => table.insert(key, entry),
            Index::Value(ref entry) => {
                //Some(entry)
                None
            }
        }
    }
}
