use crate::data::{Key, Value};
use crate::error::*;
use rocksdb::{checkpoint::Checkpoint, WriteBatch, WriteOptions, DB};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[inline(always)]
fn default_write_opts() -> WriteOptions {
    let mut res = WriteOptions::default();
    res.disable_wal(true);
    res
}

/// Backend using RocksDB as its backing store
pub struct Backend {
    db: DB,
    write_opts: WriteOptions,
    path: PathBuf,
    checkpoint_counter: u64,
}

impl Backend {
    pub fn new(path: &Path) -> Backend {
        let path: PathBuf = path.into();
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        let db = DB::open_default(path.clone()).unwrap();
        Backend {
            db,
            write_opts: default_write_opts(),
            path,
            checkpoint_counter: 0,
        }
    }

    #[inline(always)]
    pub fn put<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.db
            .put_opt(key.as_ref(), value.as_ref(), &self.write_opts)
            .map_err(|e| BrittMarieError::Insert(e.to_string()))
    }
    #[inline(always)]
    pub fn put_batch<K, V, I>(&self, kv_pairs: I) -> Result<()>
    where
        K: Key,
        V: Value,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut wb = WriteBatch::default();
        for (key, value) in kv_pairs {
            let raw_key = key.into_raw()?;
            let raw_value = value.into_raw()?;
            wb.put(raw_key, raw_value);
        }

        self.db
            .write_opt(wb, &self.write_opts)
            .map_err(|e| BrittMarieError::Insert(e.to_string()))
    }

    #[inline(always)]
    pub fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        self.db
            .get(key.as_ref())
            .map_err(|e| BrittMarieError::Read(e.to_string()))
    }
    #[inline(always)]
    pub fn checkpoint(&mut self) -> Result<()> {
        let path = self.path.join(self.checkpoint_counter.to_string());
        // taken from arcon_state
        self.db
            .flush()
            .map_err(|e| BrittMarieError::Checkpoint(e.to_string()))?;
        let checkpointer =
            Checkpoint::new(&self.db).map_err(|e| BrittMarieError::Checkpoint(e.to_string()))?;

        checkpointer
            .create_checkpoint(&path)
            .map_err(|e| BrittMarieError::Checkpoint(e.to_string()))?;

        self.checkpoint_counter += 1;
        Ok(())
    }
}
