/// Write Modes that BrittMarie provide
pub enum WriteMode {
    /// No-Copy-on-Write
    ///
    /// Only copy writes during cache eviction or an epoch checkpoint.
    NCow,
    /// Copy-on-Write
    ///
    /// Each new write will be logged. This mode is useful if you want to be able
    /// to track all updates to a specific object.
    Cow,
}

impl Default for WriteMode {
    fn default() -> Self {
        WriteMode::NCow
    }
}

#[derive(Default)]
pub struct IndexConfig {
    write_mode: WriteMode,
    // cache_policy
}
