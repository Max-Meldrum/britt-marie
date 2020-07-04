use crate::data::{Key, Value};
use crate::error::*;
use std::path::{Path, PathBuf};

cfg_if::cfg_if! {
    if #[cfg(feature = "embedded")] {
        mod rocks;
        use rocks as backend;
    } else {
        // TODO: Add support for a distributed store NDB/TIKV
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
    pub(crate) fn put<K, V>(&mut self, key: K, value: V) -> Result<()>
    where
        K: Key,
        V: Value,
    {
        let raw_key = key.into_raw()?;
        let raw_value = value.into_raw()?;
        self.backend.put(raw_key, raw_value)
    }

    /// Insert a batch of Key-Values into the store
    #[inline]
    pub(crate) fn put_batch<K, V, I>(&mut self, kv_pairs: I) -> Result<()>
    where
        K: Key,
        V: Value,
        I: IntoIterator<Item = (K, V)>,
    {
        self.backend.put_batch(kv_pairs)
    }

    #[inline]
    pub(crate) fn get<K, V>(&self, key: &K) -> Result<Option<V>>
    where
        K: Key,
        V: Value,
    {
        let raw_key = key.into_raw()?;
        let raw_opt = self.backend.get(raw_key)?;
        if let Some(raw) = raw_opt {
            let v = V::from_raw(&raw)?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }
    #[inline]
    pub fn checkpoint(&mut self) -> Result<()> {
        self.backend.checkpoint()
    }
}
