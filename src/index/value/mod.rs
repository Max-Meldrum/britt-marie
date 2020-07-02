use crate::config::IndexConfig;
use crate::data::{EvictedEntry, Key, LazyEntry, Value};
use crate::index::{IndexOps, ValueOps};
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
    /// Should be unique within the RawStore
    key: Vec<u8>,
    /// The data itself
    data: Option<V>,
    /// Reference to the RawStore
    raw_store: Rc<RefCell<RawStore>>,
}

impl<V> ValueIndex<V>
where
    V: Value,
{
    pub fn new<I>(key: I, raw_store: Rc<RefCell<RawStore>>) -> Self
    where
        I: Into<Vec<u8>>,
    {
        Self {
            key: key.into(),
            data: None,
            raw_store,
        }
    }
}

impl<V> IndexOps for ValueIndex<V>
where
    V: Value,
{
    fn persist(&self) {
        if let Some(data) = &self.data {
            self.raw_store
                .borrow_mut()
                .store(self.key.clone(), data.clone());
        }
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
    }
    #[inline(always)]
    fn rmw<F: Sized>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&mut V),
    {
        if let Some(ref mut v) = self.data.as_mut() {
            f(v);
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
