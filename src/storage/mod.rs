use bloom_filter_rs::{BloomFilter, Murmur3};
use crate::data::{Key, Value};
use std::hash::Hash;

pub struct RawStore {
    bloom: BloomFilter<Murmur3>,
}

impl RawStore {
    #[inline]
    fn store<K: Key, V: Value>(&mut self, key: K, value: V) {
    }

    #[inline]
    pub(crate) fn fetch<K: Key, V: Value>(&mut self, key: &K) -> Option<V> {
        if self.in_storage(key) {
            None // TODO
        } else {
            None
        }
    }

    /// Uses a bloom filter to check if it is worth even going to storage for the given key
    #[inline]
    fn in_storage<K: Key>(&self, key: &K) -> bool {
        self.bloom.contains(key.raw_key())
    }
}
