pub mod config;
pub mod index;

mod storage;
mod cache;
mod data;

use bloom_filter_rs::{BloomFilter, Murmur3};
use config::Config;
use data::*;
use index::hash::HashTable;
use index::Index;
use std::hash::Hash;

pub struct BrittMarie<K, V>
where
    K: Key,
    V: Value,
{
    lazy_index: Index<K, V>,
    bloom: BloomFilter<Murmur3>,
    // raw_index
    // mem_table
    // storage
}

impl<K, V> BrittMarie<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    #[inline]
    pub fn new() -> Self {
        let lazy_index = Index::Hash(HashTable::with_capacity(10));
        let bloom = BloomFilter::optimal(Murmur3, 0u64, 0.01);
        Self {
            lazy_index,
            bloom,
        }
    }
    #[inline]
    pub fn put(&mut self, key: K, value: V) {
        let entry = Entry::Lazy(LazyEntry::new(value));
        self.lazy_index.put(key, entry);
    }
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        let entry_opt = self.lazy_index.get(key);
        None
    }
    #[inline]
    fn in_storage(&self, key: &K) -> bool {
        self.bloom.contains(key.raw_key())
    }
    #[inline]
    pub fn checkpoint(&mut self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_put_test() {
        //let mut store: BrittMarie<u64, u64> = BrittMarie::new();
        //store.put(10, 100);
        //assert_eq!(store.get(10), Some(&100u64));
    }
}
