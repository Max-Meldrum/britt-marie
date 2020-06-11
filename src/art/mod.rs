pub mod raw;

use crate::art::raw::RawART;
use core::borrow::Borrow;
use core::ops::{Index, RangeBounds};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Adaptive Radix Tree
///
/// Based on the [ART paper](https://db.in.tum.de/~leis/papers/ART.pdf)
pub struct ART<K, V>
where
    K: AsRef<[u8]>,
{
    raw_art: RawART<K, V>,
}

impl<K, V> ART<K, V>
where
    K: AsRef<[u8]>,
{
    #[inline]
    pub fn new() -> Self {
        Self {
            raw_art: RawART::new(),
        }
    }

    #[inline]
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        unimplemented!();
    }

    #[inline]
    pub fn get(&mut self, key: K) -> Option<&V> {
        unimplemented!();
    }

    #[inline]
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        unimplemented!();
    }

    #[inline]
    pub fn range<T: ?Sized, R>(&self, range: R) -> Range<K, V>
    where
        T: Ord,
        K: Borrow<T>,
        R: RangeBounds<T>,
    {
        unimplemented!();
    }

    #[inline]
    pub fn range_mut<T: ?Sized, R>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        T: Ord,
        K: Borrow<T>,
        R: RangeBounds<T>,
    {
        unimplemented!();
    }
}

pub struct Range<K, V> {
    front: Option<std::marker::PhantomData<K>>,
    back: Option<std::marker::PhantomData<V>>,
}

pub struct RangeMut<'a, K: 'a, V: 'a> {
    front: Option<std::marker::PhantomData<K>>,
    back: Option<std::marker::PhantomData<V>>,

    // Be invariant in `K` and `V`
    _marker: PhantomData<&'a mut (K, V)>,
}

// Iter
// IntoIter

#[cfg(test)]
mod tests {

    #[test]
    fn simple_test() {}
}
