// Copyright (c) 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT

use core::ptr::NonNull;
use core::mem;
use core::marker::PhantomData;

cfg_if::cfg_if! {
    // Use the SSE2 implementation if possible: it allows us to scan 16 buckets
    // at once instead of 8. We don't bother with AVX since it would require
    // runtime dispatch and wouldn't gain us much anyways: the probability of
    // finding a match drops off drastically after the first few buckets.
    //
    // I attempted an implementation on ARM using NEON instructions, but it
    // turns out that most NEON instructions have multi-cycle latency, which in
    // the end outweighs any gains over the generic implementation.
    if #[cfg(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri)
    ))] {
        mod sse2;
        use sse2 as imp;
    } else {
        #[path = "generic.rs"]
        mod generic;
        use generic as imp;
    }
}

mod bitmask;

use self::bitmask::BitMask;
use self::imp::Group;


// Branch prediction hint. This is currently only available on nightly but it
// consistently improves performance by 10-15%.
#[cfg(feature = "nightly")]
use core::intrinsics::{likely, unlikely};
#[cfg(not(feature = "nightly"))]
#[inline]
fn likely(b: bool) -> bool {
    b
}
#[cfg(not(feature = "nightly"))]
#[inline]
fn unlikely(b: bool) -> bool {
    b
}

#[cfg(feature = "nightly")]
#[inline]
unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
    to.offset_from(from) as usize
}
#[cfg(not(feature = "nightly"))]
#[inline]
unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
    (to as usize - from as usize) / mem::size_of::<T>()
}

// Hash bucket
// Hash Table
// LOOK FASTER


// 2^k buckets
//
//
pub struct Table {
    //buckets: Bucket,
}

impl Table {
    pub fn new(capacity: usize, align: usize) {
        assert!(capacity.is_power_of_two());
        assert!(align.is_power_of_two());
    }
}


/// Control byte value for an empty bucket.
const EMPTY: u8 = 0b1111_1111;

/// Control byte value for a deleted bucket.
const DELETED: u8 = 0b1000_0000;

/// Checks whether a control byte represents a full bucket (top bit is clear).
#[inline]
fn is_full(ctrl: u8) -> bool {
    ctrl & 0x80 == 0
}

/// Checks whether a control byte represents a special value (top bit is set).
#[inline]
fn is_special(ctrl: u8) -> bool {
    ctrl & 0x80 != 0
}

/// Checks whether a special control value is EMPTY (just check 1 bit).
#[inline]
fn special_is_empty(ctrl: u8) -> bool {
    debug_assert!(is_special(ctrl));
    ctrl & 0x01 != 0
}

/// Primary hash function, used to select the initial bucket to probe from.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn h1(hash: u64) -> usize {
    // On 32-bit platforms we simply ignore the higher hash bits.
    hash as usize
}

/// Secondary hash function, saved in the low 7 bits of the control byte.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn h2(hash: u64) -> u8 {
    // Grab the top 7 bits of the hash. While the hash is normally a full 64-bit
    // value, some hash functions (such as FxHash) produce a usize result
    // instead, which means that the top 32 bits are 0 on 32-bit platforms.
    let hash_len = usize::min(mem::size_of::<usize>(), mem::size_of::<u64>());
    let top7 = hash >> (hash_len * 8 - 7);
    (top7 & 0x7f) as u8 // truncation
}

/// Returns the number of buckets needed to hold the given number of items,
/// taking the maximum load factor into account.
///
/// Returns `None` if an overflow occurs.
#[inline]
// Workaround for emscripten bug emscripten-core/emscripten-fastcomp#258
#[cfg_attr(target_os = "emscripten", inline(never))]
fn capacity_to_buckets(cap: usize) -> Option<usize> {
    let adjusted_cap = if cap < 8 {
        // Need at least 1 free bucket on small tables
        cap + 1
    } else {
        // Otherwise require 1/8 buckets to be empty (87.5% load)
        //
        // Be careful when modifying this, calculate_layout relies on the
        // overflow check here.
        cap.checked_mul(8)? / 7
    };

    // Any overflows will have been caught by the checked_mul. Also, any
    // rounding errors from the division above will be cleaned up by
    // next_power_of_two (which can't overflow because of the previous divison).
    Some(adjusted_cap.next_power_of_two())
}

/// Returns the maximum effective capacity for the given bucket mask, taking
/// the maximum load factor into account.
#[inline]
fn bucket_mask_to_capacity(bucket_mask: usize) -> usize {
    if bucket_mask < 8 {
        // For tables with 1/2/4/8 buckets, we always reserve one empty slot.
        // Keep in mind that the bucket mask is one less than the bucket count.
        bucket_mask
    } else {
        // For larger tables we reserve 12.5% of the slots as empty.
        ((bucket_mask + 1) / 8) * 7
    }
}


/// A reference to a hash table bucket containing a `T`.
///
/// This is usually just a pointer to the element itself. However if the element
/// is a ZST, then we instead track the index of the element in the table so
/// that `erase` works properly.
pub struct Bucket<T> {
    // Actually it is pointer to next element than element itself
    // this is needed to maintain pointer arithmetic invariants
    // keeping direct pointer to element introduces difficulty.
    // Using `NonNull` for variance and niche layout
    ptr: NonNull<T>,
}

impl<T> Clone for Bucket<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Bucket<T> {
    #[inline]
    unsafe fn from_base_index(base: NonNull<T>, index: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            // won't overflow because index must be less than length
            (index + 1) as *mut T
        } else {
            base.as_ptr().sub(index)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
    #[inline]
    unsafe fn to_base_index(&self, base: NonNull<T>) -> usize {
        if mem::size_of::<T>() == 0 {
            self.ptr.as_ptr() as usize - 1
        } else {
            offset_from(base.as_ptr(), self.ptr.as_ptr())
        }
    }
    #[inline]
    pub unsafe fn as_ptr(&self) -> *mut T {
        if mem::size_of::<T>() == 0 {
            // Just return an arbitrary ZST pointer which is properly aligned
            mem::align_of::<T>() as *mut T
        } else {
            self.ptr.as_ptr().sub(1)
        }
    }
    #[inline]
    unsafe fn next_n(&self, offset: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            (self.ptr.as_ptr() as usize + offset) as *mut T
        } else {
            self.ptr.as_ptr().sub(offset)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
    #[inline]
    pub unsafe fn drop(&self) {
        self.as_ptr().drop_in_place();
    }
    #[inline]
    pub unsafe fn read(&self) -> T {
        self.as_ptr().read()
    }
    #[inline]
    pub unsafe fn write(&self, val: T) {
        self.as_ptr().write(val);
    }
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.as_ptr()
    }
    #[inline]
    pub unsafe fn as_mut<'a>(&self) -> &'a mut T {
        &mut *self.as_ptr()
    }
    #[inline]
    pub unsafe fn copy_from_nonoverlapping(&self, other: &Self) {
        self.as_ptr().copy_from_nonoverlapping(other.as_ptr(), 1);
    }
}


/// A raw hash table with an unsafe API.
pub struct RawTable<T> {
    // Mask to get an index from a hash value. The value is one less than the
    // number of buckets in the table.
    bucket_mask: usize,

    // [Padding], T1, T2, ..., Tlast, C1, C2, ...
    //                                ^ points here
    ctrl: NonNull<u8>,

    // Number of elements that can be inserted before we need to grow the table
    growth_left: usize,

    // Number of elements in the table, only really used by len()
    items: usize,

    // Tell dropck that we own instances of T.
    marker: PhantomData<T>,
}


