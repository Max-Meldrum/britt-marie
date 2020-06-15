pub mod raw;

use crate::art::raw::RawART;
use core::borrow::Borrow;
use core::ops::{Index, RangeBounds};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Adaptive Radix Tree
///
/// Based on the [ART paper](https://db.in.tum.de/~leis/papers/ART.pdf)
pub struct ART<V> {
    raw_art: RawART<V>,
}

impl<V> ART<V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            raw_art: RawART::new(),
        }
    }

    #[inline]
    pub fn put(&mut self, key: &[u8], value: V) -> Option<V> {
        unimplemented!();
    }

    #[inline]
    pub fn get(&mut self, key: &[u8]) -> Option<&V> {
        unimplemented!();
    }

    #[inline]
    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut V> {
        unimplemented!();
    }

    #[inline]
    pub fn range<T: ?Sized, R>(&self, range: R) -> Range<V>
    where
        T: Ord,
        R: RangeBounds<T>,
    {
        unimplemented!();
    }

}

pub struct Range<V> {
    front: Option<std::marker::PhantomData<V>>,
    back: Option<std::marker::PhantomData<V>>,
}


// Iter
// IntoIter

#[cfg(test)]
mod tests {

    #[test]
    fn simple_test() {}
}
