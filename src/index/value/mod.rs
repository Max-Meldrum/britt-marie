use crate::data::Value;
use crate::error::*;
use crate::index::{IndexOps, ValueOps, WriteMode};
use crate::raw_store::RawStore;
use std::cell::RefCell;
use std::rc::Rc;

/// An Index suitable for single value operations
///
/// Examples include rolling counters, watermarks, and epochs.
pub struct ValueIndex<V>
where
    V: Value,
{
    /// Raw key for this Value
    ///
    /// Should be unique within the RawStore instance
    key: Vec<u8>,
    /// The data itself
    data: Option<V>,
    /// Write Mode
    mode: WriteMode,
    /// Reference to the RawStore
    raw_store: Rc<RefCell<RawStore>>,
}

impl<V> ValueIndex<V>
where
    V: Value,
{

    /// Creates a ValueIndex using the default lazy [WriteMode]
    pub fn new<I>(key: I, raw_store: Rc<RefCell<RawStore>>) -> Self
    where
        I: Into<Vec<u8>>,
    {
        Self::setup(key, WriteMode::default(), raw_store)
    }

    /// Creates a ValueIndex with Copy-On-Write enabled
    pub fn cow<I>(key: I, raw_store: Rc<RefCell<RawStore>>) -> Self
    where
        I: Into<Vec<u8>>,
    {
        Self::setup(key, WriteMode::Cow, raw_store)
    }

    fn setup<I>(key: I, mode: WriteMode, raw_store: Rc<RefCell<RawStore>>) -> ValueIndex<V>
    where
        I: Into<Vec<u8>>,
    {
        ValueIndex {
            key: key.into(),
            data: Some(V::default()),
            mode,
            raw_store,
        }
    }
}

impl<V> IndexOps for ValueIndex<V>
where
    V: Value,
{
    fn persist(&self) -> Result<()> {
        if let Some(data) = &self.data {
            self.raw_store
                .borrow_mut()
                .put(&self.key, data)?;
        }

        Ok(())
    }
}

impl<V> ValueOps<V> for ValueIndex<V>
where
    V: Value,
{
    #[inline(always)]
    fn get(&self) -> Option<&V> {
        self.data.as_ref()
    }
    #[inline(always)]
    fn put(&mut self, value: V) {
        self.data = Some(value);
        if self.mode.is_cow() {
            let _ = self.persist();
        }
    }
    #[inline(always)]
    fn rmw<F: Sized>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&mut V),
    {
        if let Some(ref mut v) = self.data.as_mut() {
            f(v);
            if self.mode.is_cow() {
                let _ = self.persist();
            }
        }
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let raw_store = Rc::new(RefCell::new(RawStore::new("/tmp/value")));
        let mut value_index: ValueIndex<u64> = ValueIndex::new("_myvaluekey", raw_store);
        value_index.put(10);
        assert_eq!(value_index.get(), Some(&10));
        assert_eq!(
            value_index.rmw(|v| {
                *v += 10;
            }),
            true
        );
        assert_eq!(value_index.get(), Some(&20));
    }
}
