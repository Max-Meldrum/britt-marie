use super::Value;

/// Type alias for a cache evicted entry
pub type EvictedEntry<T> = LazyEntry<T>;

/// Lazily Serialised Entry Type
pub struct LazyEntry<V>
where
    V: Value,
{
    value: V,
    meta: Metadata,
}

impl<V> LazyEntry<V>
where
    V: Value,
{
    #[inline]
    pub fn new(value: V) -> Self {
        Self {
            value,
            meta: Metadata::new(),
        }
    }
    #[inline]
    pub fn update_offset(&mut self, offset: u64) {
        self.meta.log_offset = offset;
    }
    #[inline]
    pub fn update_version(&mut self, version: u64) {
        self.meta.version = version;
    }
    #[inline]
    pub fn into_raw(self) -> RawEntry {
        RawEntry {
            value: self.value.raw_value(),
            meta: self.meta,
        }
    }
}

pub struct RawEntry {
    value: Vec<u8>,
    meta: Metadata,
}

/// BrittMarie key/value meta information
pub struct Metadata {
    /// Current version of a key/value
    version: u64,
    /// Latest known offset in log file
    log_offset: u64,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            version: 0,
            log_offset: 0,
        }
    }
}
