use crate::data::{Key, Value};
use anyhow::{Context, Result};
use std::hash::Hash;
use std::path::{Path, PathBuf};

cfg_if::cfg_if! {
    if #[cfg(feature = "embedded")] {
        mod rocks;
        use rocks as backend;
    } else {
        panic!("Only supporting embedded mode for now...");
    }
}

use backend::Backend;

pub struct RawStore {
    backend: Backend,
}

impl RawStore {
    #[cfg(feature = "embedded")]
    pub fn new(path: &str) -> RawStore {
        Self {
            backend: Backend::new(Path::new(path)),
        }
    }

    /// Insert a single Key-Value record into the store
    #[inline]
    pub(crate) fn store<K, V>(&mut self, key: K, value: V)
    where
        K: Key,
        V: Value,
    {
        let raw_key = key.into_raw().unwrap();
        let raw_value = value.into_raw().unwrap();
        self.backend.put(raw_key, raw_value).unwrap()
    }

    /// Insert a batch of Key-Values into the store
    #[inline]
    pub(crate) fn store_batch<K, V, I>(&mut self, iterator: I)
    where
        K: Key,
        V: Value,
        I: Iterator<Item = (K, V)>,
    {
        for (k, v) in iterator {}
    }

    #[inline]
    pub(crate) fn fetch<K, V>(&self, key: &K) -> Option<V>
    where
        K: Key,
        V: Value,
    {
        let raw_key = key.into_raw().unwrap();
        let raw_opt = self.backend.get(raw_key).unwrap();
        if let Some(raw) = raw_opt {
            let v = V::from_raw(&raw);
            Some(v)
        } else {
            None
        }
    }
}
