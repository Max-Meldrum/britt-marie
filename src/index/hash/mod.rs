// Copyright (c) 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT

// Modifications Copyright (c) KTH Royal Institute of Technology
// SPDX-License-Identifier: MIT

use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};

use crate::data::{Key, Value};
use crate::error::*;
use crate::index::{HashOps, IndexOps, WriteMode};

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
        mod sse2; use sse2 as imp;
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
use crate::raw_store::RawStore;
use lru::LruCache;
use std::cell::{RefCell, UnsafeCell};
use std::rc::Rc;

// Set FxHash to default as most keys tend to be small
pub type DefaultHashBuilder = fxhash::FxBuildHasher;

pub struct HashIndex<K, V>
where
    K: Key,
    V: Value,
{
    /// Hasher for the keys
    pub(crate) hash_builder: fxhash::FxBuildHasher,
    /// HashBrown's RawTable impl
    pub(crate) raw_table: UnsafeCell<RawTable<(K, V)>>,
    /// Write Mode
    mode: WriteMode,
    /// The RawStore layer where things are persisted
    raw_store: Rc<RefCell<RawStore>>,
}

#[inline]
pub(crate) fn make_hash<K: Hash + ?Sized>(hash_builder: &impl BuildHasher, val: &K) -> u64 {
    let mut state = hash_builder.build_hasher();
    val.hash(&mut state);
    state.finish()
}

impl<K, V> HashIndex<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    /// Creates a HashIndex using the default lazy WriteMode
    #[inline]
    pub fn new(capacity: usize, raw_store: Rc<RefCell<RawStore>>) -> Self {
        Self::setup(capacity, WriteMode::default(), raw_store)
    }

    /// Creates a ValueIndex with Copy-On-Write enabled
    #[inline]
    pub fn cow(capacity: usize, raw_store: Rc<RefCell<RawStore>>) -> Self {
        Self::setup(capacity, WriteMode::Cow, raw_store)
    }

    fn setup(
        capacity: usize,
        mode: WriteMode,
        raw_store: Rc<RefCell<RawStore>>,
    ) -> HashIndex<K, V> {
        HashIndex {
            hash_builder: DefaultHashBuilder::default(),
            raw_table: UnsafeCell::new(RawTable::with_capacity(capacity)),
            mode,
            raw_store,
        }
    }

    /// Internal helper function to access a RawTable
    #[inline(always)]
    fn raw_table(&self) -> &RawTable<(K, V)> {
        unsafe { &*self.raw_table.get() }
    }

    /// Internal helper function to access a mutable RawTable
    #[inline(always)]
    fn raw_table_mut(&self) -> &mut RawTable<(K, V)> {
        unsafe { &mut *self.raw_table.get() }
    }

    #[inline]
    fn insert(&self, k: K, v: V) -> Option<V> {
        let hash = make_hash(&self.hash_builder, &k);
        let table = self.raw_table_mut();
        unsafe {
            if let Some(item) = (*table).find(hash, |x| k.eq(&x.0)) {
                Some(std::mem::replace(&mut item.as_mut().1, v))
            } else {
                if (*table).is_full() {
                    // TODO: evict entry
                    None
                } else {
                    let hash_builder = &self.hash_builder;
                    (*table).insert(hash, (k, v), |x| make_hash(hash_builder, &x.0));
                    None
                }
            }
        }
    }

    #[inline]
    fn table_get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get_key_value(k).map(|(_, v)| v)
    }

    #[inline]
    fn table_get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = make_hash(&self.hash_builder, k);
        let table = self.raw_table_mut();
        table
            .find(hash, |x| k.eq(x.0.borrow()))
            .map(|item| unsafe { &mut item.as_mut().1 })
    }

    #[inline]
    fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = make_hash(&self.hash_builder, k);
        let table = self.raw_table();
        table.find(hash, |x| k.eq(x.0.borrow())).map(|item| unsafe {
            let &(ref key, ref value) = item.as_ref();
            (key, value)
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.raw_table().len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw_table().len() == 0
    }
    #[inline]
    pub fn capacity(&self) -> usize {
        self.raw_table().capacity()
    }
}

impl<K, V> IndexOps for HashIndex<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    fn persist(&self) -> Result<()> {
        if self.mode.is_cow() {
            // Every modification is already pushed to the raw store
            Ok(())
        } else {
            // TODO: Iterate over all buckets that have Modified CTRL byte.
            // NOTE: Make sure to set the CTRL byte to EVICTED
            // Thus after the iteration, all ctrl bytes should either be EMPTY or EVICTED
            Ok(())
        }
    }
}

impl<K, V> HashOps<K, V> for HashIndex<K, V>
where
    K: Key + Eq + Hash,
    V: Value,
{
    #[inline(always)]
    fn get(&self, key: &K) -> Option<&V> {
        let entry = self.table_get(key);

        // Return early if we have a match on our RawTable
        if entry.is_some() {
            return entry;
        }

        // Attempt to find the value in the RawStore
        if let Ok(entry_opt) = self.raw_store.borrow_mut().get::<K, V>(key) {
            if let Some(v) = entry_opt {
                // Insert the value back into the index
                let _ = self.insert(key.clone(), v);
                // Kinda silly but run table_get again to get the referenced value.
                // Cannot return a referenced value created in the function itself...
                self.table_get(key)
            } else {
                // The key does not exist
                return None;
            }
        } else {
            // TODO: Match on error and see if there is something that can be done?
            // otherwise panic?
            panic!("Unexpected error");
        }
    }
    #[inline(always)]
    fn put(&mut self, key: K, value: V) {
        let _ = self.insert(key, value);
    }

    #[inline(always)]
    fn rmw<F: Sized>(&mut self, key: &K, mut f: F) -> bool
    where
        F: FnMut(&mut V),
    {
        if let Some(mut entry) = self.table_get_mut(key) {
            f(&mut entry);
            if self.mode.is_cow() {
                // TODO
            }
            // indicate that the operation was successful
            return true;
        }

        // Attempt to find the value in the RawStore
        if let Ok(entry_opt) = self.raw_store.borrow_mut().get::<K, V>(key) {
            if let Some(mut value) = entry_opt {
                // run the rmw op on the value
                f(&mut value);
                if self.mode.is_cow() {
                    // TODO
                }
                // insert the value into the RawTable
                let _ = self.insert(key.clone(), value);
                // indicate that the operation was successful
                return true;
            }
        }

        // return false as the rmw operation did not modify the given key
        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/hash")));
        let mut hash_index: HashIndex<u64, u64> = HashIndex::new(128, raw_store);
        hash_index.put(1, 10);
        assert_eq!(hash_index.get(&1), Some(&10));
        assert_eq!(hash_index.get(&5), None);
        assert_eq!(
            hash_index.rmw(&1, |v| {
                *v += 5;
            }),
            true
        );
        assert_eq!(hash_index.get(&1), Some(&15));
        assert_eq!(
            hash_index.rmw(&5, |v| {
                *v += 5;
            }),
            false
        );
    }
}
