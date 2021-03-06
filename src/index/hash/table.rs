// Copyright (c) 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT

// Modifications Copyright (c) KTH Royal Institute of Technology
// SPDX-License-Identifier: MIT

use core::alloc::Layout;
use core::hint;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;
use std::alloc::{alloc, dealloc, handle_alloc_error};

use crate::hint::{likely, unlikely};
use crate::index::hash::bitmask::BitMask;
use crate::index::hash::imp::Group;

/// Augments `AllocErr` with a `CapacityOverflow` variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CollectionAllocErr {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,
    /// Error due to the allocator.
    AllocErr {
        /// The layout of the allocation request that failed.
        layout: Layout,
    },
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

/// Whether memory allocation errors should return an error or abort.
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum Fallibility {
    Fallible,
    Infallible,
}

impl Fallibility {
    /// Error to return on capacity overflow.
    #[inline]
    fn capacity_overflow(self) -> CollectionAllocErr {
        match self {
            Fallibility::Fallible => CollectionAllocErr::CapacityOverflow,
            Fallibility::Infallible => panic!("Hash table capacity overflow"),
        }
    }

    /// Error to return on allocation error.
    #[inline]
    fn alloc_err(self, layout: Layout) -> CollectionAllocErr {
        match self {
            Fallibility::Fallible => CollectionAllocErr::AllocErr { layout },
            Fallibility::Infallible => handle_alloc_error(layout),
        }
    }
}

/// Control byte value for an empty bucket.
pub(crate) const EMPTY: u8 = 0b1111_1111;
/// Control byte value for a deleted bucket.
const DELETED: u8 = 0b1000_0000;

/// Meta byte value for a modified bucket.
const MODIFIED: u8 = 0b1000_0000;
const MODIFIED_TOUCHED: u8 = 0b1100_0000;

/// Meta byte value for a safe bucket.
const SAFE: u8 = 0b0000_0000;
const SAFE_TOUCHED: u8 = 0b0100_0000;

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

/// Checks whether a meta byte represents a modified bucket (top bit is set).
#[inline]
#[allow(dead_code)]
fn is_modified(meta: u8) -> bool {
    meta & 0x80 != 0
}

/// Checks whether a meta byte represents a safe bucket (top bit is clear).
#[inline]
fn is_safe(meta: u8) -> bool {
    meta & 0x80 == 0
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

/// Probe sequence based on triangular numbers, which is guaranteed (since our
/// table size is a power of two) to visit every group of elements exactly once.
///
/// A triangular probe has us jump by 1 more group every time. So first we
/// jump by 1 group (meaning we just continue our linear scan), then 2 groups
/// (skipping over 1 group), then 3 groups (skipping over 2 groups), and so on.
///
/// Proof that the probe will visit every group in the table:
/// <https://fgiesen.wordpress.com/2015/02/22/triangular-numbers-mod-2n/>
struct ProbeSeq {
    bucket_mask: usize,
    pos: usize,
    stride: usize,
}

impl Iterator for ProbeSeq {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        if self.stride >= self.bucket_mask {
            return None;
        }

        let result = self.pos;
        self.stride += Group::WIDTH;
        self.pos += self.stride;
        self.pos &= self.bucket_mask;
        Some(result)
    }
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

/// Returns a Layout which describes the allocation required for a hash table,
/// and the offset of the control bytes in the allocation.
/// (the offset is also one past last element of buckets)
///
/// Returns `None` if an overflow occurs.
#[inline]
#[cfg(feature = "nightly")]
fn calculate_layout<T>(buckets: usize) -> Option<(Layout, usize)> {
    debug_assert!(buckets.is_power_of_two());

    // Array of buckets
    let data = Layout::array::<T>(buckets).ok()?;

    // Array of control bytes. This must be aligned to the group size.
    //
    // We add `Group::WIDTH` control bytes at the end of the array which
    // replicate the bytes at the start of the array and thus avoids the need to
    // perform bounds-checking while probing.
    //
    // There is no possible overflow here since buckets is a power of two and
    // Group::WIDTH is a small number.
    let ctrl = unsafe { Layout::from_size_align_unchecked(buckets + Group::WIDTH, Group::WIDTH) };

    data.extend(ctrl).ok()
}

/// Returns a Layout which describes the allocation required for a hash table,
/// and the offset of the control bytes in the allocation.
/// (the offset is also one past last element of buckets)
///
/// Returns `None` if an overflow occurs.
#[inline]
#[cfg(not(feature = "nightly"))]
fn calculate_layout<T>(buckets: usize) -> Option<(Layout, usize)> {
    debug_assert!(buckets.is_power_of_two());

    // Manual layout calculation since Layout methods are not yet stable.
    let ctrl_align = usize::max(mem::align_of::<T>(), Group::WIDTH);
    let ctrl_offset = mem::size_of::<T>()
        .checked_mul(buckets)?
        .checked_add(ctrl_align - 1)?
        & !(ctrl_align - 1);
    let len = ctrl_offset.checked_add(buckets + Group::WIDTH)?;

    Some((
        unsafe { Layout::from_size_align_unchecked(len, ctrl_align) },
        ctrl_offset,
    ))
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
}

/// A raw hash table with an unsafe API.
pub struct RawTable<T> {
    // Mask to get an index from a hash value. The value is one less than the
    // number of buckets in the table.
    bucket_mask: usize,

    // [Padding], T1, T2, ..., Tlast, C1, C2, ...
    //                                ^ points here
    ctrl: NonNull<u8>,

    meta: NonNull<u8>,

    // Number of elements that can be inserted before we need to grow the table
    growth_left: usize,

    // Number of elements in the table, only really used by len()
    items: usize,

    mod_counter: usize,
    // e.g., only 30% of ctrl full bytes may be set as modified
    //
    mod_limit: usize,

    // Tell dropck that we own instances of T.
    marker: PhantomData<T>,
}

impl<T> RawTable<T> {
    /// Creates a new empty hash table without allocating any memory.
    ///
    /// In effect this returns a table with exactly 1 bucket. However we can
    /// leave the data pointer dangling since that bucket is never written to
    /// due to our load factor forcing us to always have at least 1 free bucket.
    #[inline]
    pub fn new() -> Self {
        Self {
            // Be careful to cast the entire slice to a raw pointer.
            ctrl: unsafe { NonNull::new_unchecked(Group::static_empty().as_ptr() as *mut u8) },
            meta: unsafe { NonNull::new_unchecked(Group::static_empty().as_ptr() as *mut u8) },
            bucket_mask: 0,
            items: 0,
            growth_left: 0,
            mod_counter: 0,
            mod_limit: 0,
            marker: PhantomData,
        }
    }

    /// Allocates a new hash table with the given number of buckets.
    ///
    /// The control bytes are left uninitialized.
    #[inline]
    unsafe fn new_uninitialized(
        buckets: usize,
        mod_factor: f32,
        fallability: Fallibility,
    ) -> Result<Self, CollectionAllocErr> {
        debug_assert!(buckets.is_power_of_two());
        let (layout, ctrl_offset) =
            calculate_layout::<T>(buckets).ok_or_else(|| fallability.capacity_overflow())?;
        let ctrl_ptr = NonNull::new(alloc(layout)).ok_or_else(|| fallability.alloc_err(layout))?;
        let meta_ptr = NonNull::new(alloc(layout)).ok_or_else(|| fallability.alloc_err(layout))?;
        let ctrl = NonNull::new_unchecked(ctrl_ptr.as_ptr().add(ctrl_offset));
        let meta = NonNull::new_unchecked(meta_ptr.as_ptr().add(ctrl_offset));
        let growth_left = bucket_mask_to_capacity(buckets - 1);

        Ok(Self {
            ctrl,
            meta,
            bucket_mask: buckets - 1,
            items: 0,
            mod_counter: 0,
            mod_limit: (growth_left as f32 * mod_factor) as usize,
            growth_left,
            marker: PhantomData,
        })
    }

    /// Attempts to allocate a new hash table with at least enough capacity
    /// for inserting the given number of elements without reallocating.
    fn try_with_capacity(
        capacity: usize,
        mod_factor: f32,
        fallability: Fallibility,
    ) -> Result<Self, CollectionAllocErr> {
        if capacity == 0 {
            Ok(Self::new())
        } else {
            unsafe {
                let buckets =
                    capacity_to_buckets(capacity).ok_or_else(|| fallability.capacity_overflow())?;
                let result = Self::new_uninitialized(buckets, mod_factor, fallability)?;
                result.ctrl(0).write_bytes(EMPTY, result.num_ctrl_bytes());
                result.meta(0).write_bytes(SAFE, result.num_ctrl_bytes());

                Ok(result)
            }
        }
    }

    /// Allocates a new hash table with at least enough capacity for inserting
    /// the given number of elements without reallocating.
    pub fn with_capacity(capacity: usize, mod_factor: f32) -> Self {
        assert!(
            mod_factor > 0.0 && mod_factor <= 0.9,
            "Modification factor needs to be set between 0.0 and 0.9"
        );
        Self::try_with_capacity(capacity, mod_factor, Fallibility::Infallible)
            .unwrap_or_else(|_| unsafe { hint::unreachable_unchecked() })
    }

    /// Deallocates the table without dropping any entries.
    #[inline]
    unsafe fn free_buckets(&mut self) {
        let (layout, ctrl_offset) =
            calculate_layout::<T>(self.buckets()).unwrap_or_else(|| hint::unreachable_unchecked());
        dealloc(self.ctrl.as_ptr().sub(ctrl_offset), layout);
    }

    /// Returns pointer to one past last element of data table.
    #[inline]
    pub unsafe fn data_end(&self) -> NonNull<T> {
        NonNull::new_unchecked(self.ctrl.as_ptr() as *mut T)
    }

    /// Returns pointer to start of data table.
    #[inline]
    #[cfg(feature = "nightly")]
    pub unsafe fn data_start(&self) -> *mut T {
        self.data_end().as_ptr().wrapping_sub(self.buckets())
    }

    /// Returns the index of a bucket from a `Bucket`.
    #[inline]
    pub unsafe fn _bucket_index(&self, bucket: &Bucket<T>) -> usize {
        bucket.to_base_index(self.data_end())
    }

    /// Returns a pointer to a control byte.
    #[inline]
    unsafe fn ctrl(&self, index: usize) -> *mut u8 {
        debug_assert!(index < self.num_ctrl_bytes());
        self.ctrl.as_ptr().add(index)
    }

    /// Returns a pointer to a meta byte
    #[inline]
    unsafe fn meta(&self, index: usize) -> *mut u8 {
        debug_assert!(index < self.num_ctrl_bytes());
        self.meta.as_ptr().add(index)
    }

    /// Returns a pointer to an element in the table.
    #[inline]
    pub unsafe fn bucket(&self, index: usize) -> Bucket<T> {
        debug_assert_ne!(self.bucket_mask, 0);
        debug_assert!(index < self.buckets());
        Bucket::from_base_index(self.data_end(), index)
    }

    #[inline]
    pub unsafe fn erase_by_index(&mut self, index: usize) {
        debug_assert!(is_full(*self.ctrl(index)));
        let index_before = index.wrapping_sub(Group::WIDTH) & self.bucket_mask;
        let empty_before = Group::load(self.ctrl(index_before)).match_empty();
        let empty_after = Group::load(self.ctrl(index)).match_empty();

        // If we are inside a continuous block of Group::WIDTH full or deleted
        // cells then a probe window may have seen a full block when trying to
        // insert. We therefore need to keep that block non-empty so that
        // lookups will continue searching to the next probe window.
        //
        // Note that in this context `leading_zeros` refers to the bytes at the
        // end of a group, while `trailing_zeros` refers to the bytes at the
        // begining of a group.
        let ctrl = if empty_before.leading_zeros() + empty_after.trailing_zeros() >= Group::WIDTH {
            DELETED
        } else {
            self.growth_left += 1;
            EMPTY
        };
        self.set_ctrl(index, ctrl);
        self.items -= 1;
    }

    /// Returns an iterator for a probe sequence on the table.
    ///
    /// This iterator never terminates, but is guaranteed to visit each bucket
    /// group exactly once. The loop using `probe_seq` must terminate upon
    /// reaching a group containing an empty bucket.
    #[inline]
    fn probe_seq(&self, hash: u64) -> ProbeSeq {
        ProbeSeq {
            bucket_mask: self.bucket_mask,
            pos: h1(hash) & self.bucket_mask,
            stride: 0,
        }
    }
    /// Sets a meta byte
    #[inline]
    unsafe fn set_meta(&self, index: usize, meta: u8) {
        // Using same logic as set_ctrl
        let index2 = ((index.wrapping_sub(Group::WIDTH)) & self.bucket_mask) + Group::WIDTH;
        *self.meta(index) = meta;
        *self.meta(index2) = meta;
    }

    /// Sets a control byte, and possibly also the replicated control byte at
    /// the end of the array.
    #[inline]
    unsafe fn set_ctrl(&self, index: usize, ctrl: u8) {
        // Replicate the first Group::WIDTH control bytes at the end of
        // the array without using a branch:
        // - If index >= Group::WIDTH then index == index2.
        // - Otherwise index2 == self.bucket_mask + 1 + index.
        //
        // The very last replicated control byte is never actually read because
        // we mask the initial index for unaligned loads, but we write it
        // anyways because it makes the set_ctrl implementation simpler.
        //
        // If there are fewer buckets than Group::WIDTH then this code will
        // replicate the buckets at the end of the trailing group. For example
        // with 2 buckets and a group size of 4, the control bytes will look
        // like this:
        //
        //     Real    |             Replicated
        // ---------------------------------------------
        // | [A] | [B] | [EMPTY] | [EMPTY] | [A] | [B] |
        // ---------------------------------------------
        let index2 = ((index.wrapping_sub(Group::WIDTH)) & self.bucket_mask) + Group::WIDTH;

        *self.ctrl(index) = ctrl;
        *self.ctrl(index2) = ctrl;
    }

    /// Searches for an empty or deleted bucket which is suitable for inserting
    /// a new element.
    ///
    /// There must be at least 1 empty bucket in the table.
    #[inline]
    fn find_insert_slot(&self, hash: u64) -> usize {
        for pos in self.probe_seq(hash) {
            unsafe {
                let group = Group::load(self.ctrl(pos));
                if let Some(bit) = group.match_empty_or_deleted().lowest_set_bit() {
                    let result = (pos + bit) & self.bucket_mask;

                    // In tables smaller than the group width, trailing control
                    // bytes outside the range of the table are filled with
                    // EMPTY entries. These will unfortunately trigger a
                    // match, but once masked may point to a full bucket that
                    // is already occupied. We detect this situation here and
                    // perform a second scan starting at the begining of the
                    // table. This second scan is guaranteed to find an empty
                    // slot (due to the load factor) before hitting the trailing
                    // control bytes (containing EMPTY).
                    if unlikely(is_full(*self.ctrl(result))) {
                        debug_assert!(self.bucket_mask < Group::WIDTH);
                        debug_assert_ne!(pos, 0);
                        return Group::load_aligned(self.ctrl(0))
                            .match_empty_or_deleted()
                            .lowest_set_bit_nonzero();
                    } else {
                        return result;
                    }
                }
            }
        }
        // probe_seq does return but it should never reach this
        // as there will always be an empty bucket available...
        unreachable!();
    }

    /// Searches for an index that is suitable to "erase"
    ///
    /// There must always be at least 1 SAFE bucket in the table
    #[inline]
    fn clear_safe_bucket(&mut self, hash: u64) {
        for pos in self.probe_seq(hash) {
            unsafe {
                // In the best case we find a bucket in the first group
                // as the insert after will be jumping to the same group.
                let group = Group::load(self.meta(pos));

                // First attempt to match on SAFE meta byte within the group.
                // Otherwise, fall back to SAFE_TOUCHED.
                let bit_opt = {
                    let safe = group.match_byte(SAFE).lowest_set_bit();
                    if safe.is_none() {
                        group.match_byte(SAFE_TOUCHED).lowest_set_bit()
                    } else {
                        safe
                    }
                };

                if let Some(bit) = bit_opt {
                    let result = (pos + bit) & self.bucket_mask;

                    let index = {
                        if unlikely(is_modified(*self.meta(result))) {
                            debug_assert!(self.bucket_mask < Group::WIDTH);
                            debug_assert_ne!(pos, 0);
                            Group::load_aligned(self.meta(0))
                                .match_empty_or_deleted()
                                .lowest_set_bit_nonzero()
                        } else {
                            result
                        }
                    };
                    self.erase_by_index(index);
                    // Set its meta byte to SAFE in case it was SAFE_TOUCHED
                    self.set_meta(index, SAFE);
                    // We are done, return...
                    return;
                }
            }
        }
    }

    /// Searches for a modified bucket to evict
    ///
    /// Safety: Should only be called when there is at least 1 modified bucket in the table
    #[inline]
    #[allow(unused_assignments)]
    pub(crate) unsafe fn evict_mod_bucket(&mut self, hash: u64) -> Bucket<T> {
        debug_assert_ne!(self.mod_counter, 0);
        for pos in self.probe_seq(hash) {
            let group = Group::load(self.meta(pos));

            // First match for MODIFIED meta byte. If that fails
            // then check for MODIFIED_TOUCHED within the same group.
            let bit_match = {
                let modified = group.match_byte(MODIFIED).lowest_set_bit();
                if modified.is_none() {
                    group.match_byte(MODIFIED_TOUCHED).lowest_set_bit()
                } else {
                    modified
                }
            };

            if let Some(bit) = bit_match {
                let result = (pos + bit) & self.bucket_mask;

                // In tables smaller than the group width, trailing meta
                // bytes outside the range of the table are filled with
                // EMPTY entries. These will unfortunately trigger a
                // match, but once masked may point to a full bucket that
                // is already occupied. We detect this situation here and
                // perform a second scan starting at the begining of the
                // table. This second scan is guaranteed to find an empty
                // slot (due to the load factor) before hitting the trailing
                // control bytes (containing EMPTY).
                let index = {
                    if unlikely(is_safe(*self.meta(result))) {
                        debug_assert!(self.bucket_mask < Group::WIDTH);
                        debug_assert_ne!(pos, 0);
                        Group::load_aligned(self.meta(0))
                            .match_empty_or_deleted()
                            .lowest_set_bit_nonzero()
                    } else {
                        result
                    }
                };

                let bucket = self.bucket(index);

                if self.growth_left == 0 {
                  // If there is no space left, then actually "erase" it.
                  self.erase_by_index(index);
                }

                // Set bucket to safe
                self.set_meta(index, SAFE);
                self.mod_counter -= 1;

                return bucket;
            }
        }

        // probe_seq never returns.
        unreachable!();
    }

    /// Inserts a new element into the table.
    ///
    /// This does not check if the given element already exists in the table.
    #[inline]
    pub fn insert(&mut self, hash: u64, value: T) -> Bucket<T> {
        unsafe {
            if unlikely(self.growth_left == 0) {
                self.clear_safe_bucket(hash);
            }

            let index = self.find_insert_slot(hash);
            let ctrl = *self.ctrl(index);

            let bucket = self.bucket(index);
            self.growth_left = self
                .growth_left
                .saturating_sub(special_is_empty(ctrl) as usize);
            self.set_ctrl(index, h2(hash));
            self.set_meta(index, MODIFIED);
            bucket.write(value);
            self.items += 1;
            self.mod_counter += 1;
            bucket
        }
    }

    /// Searches for an element in the table.
    ///
    /// Similar to the find function, but we use it for mutable finds and thus set
    /// bucket's meta byte to MODIFIED_TOUCHED.
    ///
    /// Note that we simply assume that the caller will modify the underlying value.
    #[inline]
    pub fn find_mut(&mut self, hash: u64, mut eq: impl FnMut(&T) -> bool) -> Option<Bucket<T>> {
        unsafe {
            for pos in self.probe_seq(hash) {
                let group = Group::load(self.ctrl(pos));
                for bit in group.match_byte(h2(hash)) {
                    let index = (pos + bit) & self.bucket_mask;
                    let bucket = self.bucket(index);
                    if likely(eq(bucket.as_ref())) {
                        // If the meta was safe, then increase modification
                        // counter as we are setting the meta to MODIFIED_TOUCHED.
                        if is_safe(*self.meta(index)) {
                            self.mod_counter += 1;
                        }

                        self.set_meta(index, MODIFIED_TOUCHED);
                        return Some(bucket);
                    }
                }
            }

            return None;
        }
    }

    /// Searches for an element in the table.
    #[inline]
    pub fn find(&self, hash: u64, mut eq: impl FnMut(&T) -> bool) -> Option<Bucket<T>> {
        unsafe {
            for pos in self.probe_seq(hash) {
                let group = Group::load(self.ctrl(pos));
                for bit in group.match_byte(h2(hash)) {
                    let index = (pos + bit) & self.bucket_mask;
                    let bucket = self.bucket(index);
                    if likely(eq(bucket.as_ref())) {
                        if is_safe(*self.meta(index)) {
                            self.set_meta(index, SAFE_TOUCHED);
                        }
                        return Some(bucket);
                    }
                }
            }
        }
        return None;
    }

    /// Returns the number of elements the map can hold without reallocating.
    ///
    /// This number is a lower bound; the table might be able to hold
    /// more, but is guaranteed to be able to hold at least this many.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.items + self.growth_left
    }

    /// Returns the number of elements in the table.
    #[inline]
    pub fn len(&self) -> usize {
        self.items
    }

    /// Returns whether the table is above allowed modification threshold
    #[inline]
    pub fn above_mod_threshold(&self) -> bool {
        self.mod_counter >= self.mod_limit
    }

    /// Returns the number of buckets in the table.
    #[inline]
    pub fn buckets(&self) -> usize {
        self.bucket_mask + 1
    }

    /// Returns the number of control bytes in the table.
    #[inline]
    fn num_ctrl_bytes(&self) -> usize {
        self.bucket_mask + 1 + Group::WIDTH
    }

    /// Returns whether this table points to the empty singleton with a capacity
    /// of 0.
    #[inline]
    fn is_empty_singleton(&self) -> bool {
        self.bucket_mask == 0
    }

    /// Returns the amount of amount of allowed modifications
    #[inline]
    pub fn mod_limit(&self) -> usize {
        self.mod_limit
    }

    /// Returns an iterator over every element in the table that has a meta byte set as MODIFIED.
    /// Note that this iterator will reset every MODIFIED byte to SAFE.
    #[inline]
    pub unsafe fn iter_modified(&mut self) -> ModifiedIterator<T> {
        let data = Bucket::from_base_index(self.data_end(), 0);
        self.mod_counter = 0;
        ModifiedIterator {
            iter: RawIterModified::new(
                self.ctrl.as_ptr(),
                self.meta.as_ptr(),
                data,
                self.buckets(),
            ),
        }
    }

    /// Returns an iterator over every element in the table. It is up to
    /// the caller to ensure that the `RawTable` outlives the `RawIter`.
    /// Because we cannot make the `next` method unsafe on the `RawIter`
    /// struct, we have to make the `iter` method unsafe.
    #[inline]
    pub unsafe fn iter(&self) -> RawIter<T> {
        let data = Bucket::from_base_index(self.data_end(), 0);
        RawIter {
            iter: RawIterRange::new(self.ctrl.as_ptr(), data, self.buckets()),
            items: self.items,
        }
    }
}

unsafe impl<T> Send for RawTable<T> where T: Send {}
unsafe impl<T> Sync for RawTable<T> where T: Sync {}

#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] T> Drop for RawTable<T> {
    #[inline]
    fn drop(&mut self) {
        if !self.is_empty_singleton() {
            unsafe {
                if mem::needs_drop::<T>() {
                    for item in self.iter() {
                        item.drop();
                    }
                }
                self.free_buckets();
            }
        }
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> Drop for RawTable<T> {
    #[inline]
    fn drop(&mut self) {
        if !self.is_empty_singleton() {
            unsafe {
                if mem::needs_drop::<T>() {
                    for item in self.iter() {
                        item.drop();
                    }
                }
                self.free_buckets();
            }
        }
    }
}

/// Iterator over modified table buckets
pub(crate) struct RawIterModified<T> {
    // Mask of full buckets in the current group. Bits are cleared from this
    // mask as each element is processed.
    current_group: BitMask,

    // Pointer to the buckets for the current group.
    data: Bucket<T>,

    ctrl: NonNull<u8>,
    meta: NonNull<u8>,

    // Pointer to the next group of meta bytes
    // Must be aligned to the group size.
    next_meta: *const u8,
    // Pointer one past the last meta byte of this range.
    meta_end: *const u8,
}
impl<T> RawIterModified<T> {
    /// Returns a `RawIterModified` covering modified buckets of the table
    ///
    /// The control & meta byte address must be aligned to the group size.
    #[inline]
    unsafe fn new(ctrl: *const u8, meta: *const u8, data: Bucket<T>, len: usize) -> Self {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(meta as usize % Group::WIDTH, 0);
        let meta_end = meta.add(len);

        // Load the first group and advance meta to point to the next group
        let current_group = Group::load_aligned(meta).match_modified();
        let next_meta = meta.add(Group::WIDTH);
        let ctrl = NonNull::new_unchecked(ctrl as *mut u8);
        let meta = NonNull::new_unchecked(meta as *mut u8);

        Self {
            current_group,
            data,
            ctrl,
            meta,
            next_meta,
            meta_end,
        }
    }
}

impl<T> Clone for RawIterModified<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            ctrl: self.ctrl,
            meta: self.meta,
            next_meta: self.next_meta,
            current_group: self.current_group,
            meta_end: self.meta_end,
        }
    }
}

impl<T> Iterator for RawIterModified<T> {
    type Item = Bucket<T>;

    #[inline]
    fn next(&mut self) -> Option<Bucket<T>> {
        unsafe {
            loop {
                if let Some(index) = self.current_group.lowest_set_bit() {
                    self.current_group = self.current_group.remove_lowest_bit();
                    let bucket = self.data.next_n(index);
                    // Get index for this bucket
                    let bucket_index =
                        bucket.to_base_index(NonNull::new_unchecked(self.ctrl.as_ptr() as *mut T));
                    // Set the meta byte of the index to SAFE
                    std::ptr::write(self.meta.as_ptr().add(bucket_index), SAFE);
                    return Some(bucket);
                }

                if self.next_meta >= self.meta_end {
                    return None;
                }

                // We might read past self.end up to the next group boundary,
                // but this is fine because it only occurs on tables smaller
                // than the group size where the trailing control bytes are all
                // EMPTY. On larger tables self.end is guaranteed to be aligned
                // to the group size (since tables are power-of-two sized).
                self.current_group = Group::load_aligned(self.next_meta).match_modified();
                self.data = self.data.next_n(Group::WIDTH);
                self.next_meta = self.next_meta.add(Group::WIDTH);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // We don't have an item count, so just guess based on the range size.
        (
            0,
            Some(unsafe { offset_from(self.meta_end, self.next_meta) + Group::WIDTH }),
        )
    }
}

/// Iterator over a sub-range of a table. Unlike `RawIter` this iterator does
/// not track an item count.
pub(crate) struct RawIterRange<T> {
    // Mask of full buckets in the current group. Bits are cleared from this
    // mask as each element is processed.
    current_group: BitMask,

    // Pointer to the buckets for the current group.
    data: Bucket<T>,

    // Pointer to the next group of control bytes,
    // Must be aligned to the group size.
    next_ctrl: *const u8,

    // Pointer one past the last control byte of this range.
    end: *const u8,
}

impl<T> RawIterRange<T> {
    /// Returns a `RawIterRange` covering a subset of a table.
    ///
    /// The control byte address must be aligned to the group size.
    #[inline]
    unsafe fn new(ctrl: *const u8, data: Bucket<T>, len: usize) -> Self {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(ctrl as usize % Group::WIDTH, 0);
        let end = ctrl.add(len);

        // Load the first group and advance ctrl to point to the next group
        let current_group = Group::load_aligned(ctrl).match_full();
        let next_ctrl = ctrl.add(Group::WIDTH);

        Self {
            current_group,
            data,
            next_ctrl,
            end,
        }
    }

    /// Splits a `RawIterRange` into two halves.
    ///
    /// Returns `None` if the remaining range is smaller than or equal to the
    /// group width.
    #[inline]
    #[cfg(feature = "rayon")]
    pub(crate) fn split(mut self) -> (Self, Option<RawIterRange<T>>) {
        unsafe {
            if self.end <= self.next_ctrl {
                // Nothing to split if the group that we are current processing
                // is the last one.
                (self, None)
            } else {
                // len is the remaining number of elements after the group that
                // we are currently processing. It must be a multiple of the
                // group size (small tables are caught by the check above).
                let len = offset_from(self.end, self.next_ctrl);
                debug_assert_eq!(len % Group::WIDTH, 0);

                // Split the remaining elements into two halves, but round the
                // midpoint down in case there is an odd number of groups
                // remaining. This ensures that:
                // - The tail is at least 1 group long.
                // - The split is roughly even considering we still have the
                //   current group to process.
                let mid = (len / 2) & !(Group::WIDTH - 1);

                let tail = Self::new(
                    self.next_ctrl.add(mid),
                    self.data.next_n(Group::WIDTH).next_n(mid),
                    len - mid,
                );
                debug_assert_eq!(
                    self.data.next_n(Group::WIDTH).next_n(mid).ptr,
                    tail.data.ptr
                );
                debug_assert_eq!(self.end, tail.end);
                self.end = self.next_ctrl.add(mid);
                debug_assert_eq!(self.end.add(Group::WIDTH), tail.next_ctrl);
                (self, Some(tail))
            }
        }
    }
}

// We make raw iterators unconditionally Send and Sync, and let the PhantomData
// in the actual iterator implementations determine the real Send/Sync bounds.
unsafe impl<T> Send for RawIterRange<T> {}
unsafe impl<T> Sync for RawIterRange<T> {}

impl<T> Clone for RawIterRange<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            next_ctrl: self.next_ctrl,
            current_group: self.current_group,
            end: self.end,
        }
    }
}

impl<T> Iterator for RawIterRange<T> {
    type Item = Bucket<T>;

    #[inline]
    fn next(&mut self) -> Option<Bucket<T>> {
        unsafe {
            loop {
                if let Some(index) = self.current_group.lowest_set_bit() {
                    self.current_group = self.current_group.remove_lowest_bit();
                    return Some(self.data.next_n(index));
                }

                if self.next_ctrl >= self.end {
                    return None;
                }

                // We might read past self.end up to the next group boundary,
                // but this is fine because it only occurs on tables smaller
                // than the group size where the trailing control bytes are all
                // EMPTY. On larger tables self.end is guaranteed to be aligned
                // to the group size (since tables are power-of-two sized).
                self.current_group = Group::load_aligned(self.next_ctrl).match_full();
                self.data = self.data.next_n(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // We don't have an item count, so just guess based on the range size.
        (
            0,
            Some(unsafe { offset_from(self.end, self.next_ctrl) + Group::WIDTH }),
        )
    }
}

impl<T> FusedIterator for RawIterRange<T> {}

/// Iterator which returns a raw pointer to every full bucket in the table.
pub struct ModifiedIterator<T> {
    pub(crate) iter: RawIterModified<T>,
}

impl<T> Clone for ModifiedIterator<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
        }
    }
}

impl<T> Iterator for ModifiedIterator<T> {
    type Item = Bucket<T>;

    #[inline]
    fn next(&mut self) -> Option<Bucket<T>> {
        if let Some(b) = self.iter.next() {
            Some(b)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<T> ExactSizeIterator for ModifiedIterator<T> {}
impl<T> FusedIterator for ModifiedIterator<T> {}

/// Iterator which returns a raw pointer to every full bucket in the table.
pub struct RawIter<T> {
    pub(crate) iter: RawIterRange<T>,
    items: usize,
}

impl<T> Clone for RawIter<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            items: self.items,
        }
    }
}

impl<T> Iterator for RawIter<T> {
    type Item = Bucket<T>;

    #[inline]
    fn next(&mut self) -> Option<Bucket<T>> {
        if let Some(b) = self.iter.next() {
            self.items -= 1;
            Some(b)
        } else {
            // We don't check against items == 0 here to allow the
            // compiler to optimize away the item count entirely if the
            // iterator length is never queried.
            debug_assert_eq!(self.items, 0);
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items, Some(self.items))
    }
}

impl<T> ExactSizeIterator for RawIter<T> {}
impl<T> FusedIterator for RawIter<T> {}
