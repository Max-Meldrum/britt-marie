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

pub mod config;
pub mod index;

mod cache;
/// Data types used by the crate
mod data;
/// Set of compiler hints
mod hint;
mod storage;
