/// Write Modes that BrittMarie provide
#[derive(PartialEq)]
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
}

impl IndexConfig {
    #[inline(always)]
    pub fn is_cow(&self) -> bool {
        self.write_mode == WriteMode::Cow
    }
}
