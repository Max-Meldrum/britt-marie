// Copyright (c) 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT

// Modifications Copyright (c) KTH Royal Institute of Technology
// SPDX-License-Identifier: MIT

use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};

use crate::data::{Entry, Key, LazyEntry, Value};

cfg_if::cfg_if! {
    // Use the SSE2 implementation if possible: it allows us to scan 16 buckets
    // at once instead of 8. We don't bother with AVX since it would require
    // runtime dispatch and wouldn't gain us much anyways: the probability of
    // finding a match drops off drastically after the first few buckets.
    //
    // I attempted an implementation on ARM using NEON instructions, but it
    // turns out that most NEON instructions have multi-cycle latency, which in
    // the end outweighs any gains over the generic implementation.
    if #[cfg(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri)
    ))] {
        mod sse2;
        use sse2 as imp;
    } else {
        #[path = "generic.rs"]
        mod generic;
        use generic as imp;
    }
}

mod bitmask;
mod table;

use self::bitmask::BitMask;
use self::imp::Group;
use self::table::RawTable;

// Set FxHash to default as most keys tend to be small
pub type DefaultHashBuilder = fxhash::FxBuildHasher;

pub struct HashTable<K, V>
where
    K: Key,
    V: Value,
{
    pub(crate) hash_builder: fxhash::FxBuildHasher,
    pub(crate) table: RawTable<(K, Entry<V>)>,
    // cache? lru/tinylfu
}

#[inline]
pub(crate) fn make_hash<K: Hash + ?Sized>(hash_builder: &impl BuildHasher, val: &K) -> u64 {
    let mut state = hash_builder.build_hasher();
    val.hash(&mut state);
    state.finish()
}

impl<K, V> HashTable<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            hash_builder: DefaultHashBuilder::default(),
            table: RawTable::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: Entry<V>) -> Option<Entry<V>> {
        unsafe {
            let hash = make_hash(&self.hash_builder, &k);
            if let Some(item) = self.table.find(hash, |x| k.eq(&x.0)) {
                Some(std::mem::replace(&mut item.as_mut().1, v))
            } else {
                let hash_builder = &self.hash_builder;
                self.table
                    .insert(hash, (k, v), |x| make_hash(hash_builder, &x.0));
                None
            }
        }
    }

    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Entry<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get_key_value(k).map(|(_, v)| v)
    }

    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Entry<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = make_hash(&self.hash_builder, k);
        self.table
            .find(hash, |x| k.eq(x.0.borrow()))
            .map(|item| unsafe { &mut item.as_mut().1 })
    }

    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &Entry<V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = make_hash(&self.hash_builder, k);
        self.table
            .find(hash, |x| k.eq(x.0.borrow()))
            .map(|item| unsafe {
                let &(ref key, ref value) = item.as_ref();
                (key, value)
            })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }
}
