use anyhow::{Context, Result};
use rocksdb::{
    checkpoint::Checkpoint, ColumnFamily, ColumnFamilyDescriptor, DBPinnableSlice, Options,
    SliceTransform, WriteBatch, WriteOptions, DB,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Backend {
    db: DB,
    path: PathBuf,
}

impl Backend {
    pub fn new(path: &Path) -> Backend {
        let path: PathBuf = path.into();
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        let db = DB::open_default(path.clone()).unwrap();
        Backend { db, path }
    }

    #[inline(always)]
    pub fn put<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.db
            .put(key.as_ref(), value.as_ref())
            .with_context(|| "hej")
    }
    #[inline(always)]
    pub fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        self.db.get(key.as_ref()).with_context(|| "hej")
    }
}
