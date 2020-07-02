#![cfg_attr(
    feature = "nightly",
    feature(
        alloc_layout_extra,
        allocator_api,
        ptr_offset_from,
        test,
        core_intrinsics,
        dropck_eyepatch,
        specialization,
    )
)]

/// Index Config
pub mod config;
/// Available Lazy State Indexes
pub mod index;
/// Backing Stores
pub mod raw_store;

/// Data types used by the crate
mod data;
/// Set of compiler hints
mod hint;
