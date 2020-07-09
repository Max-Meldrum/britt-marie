//! Britt-Marie is a special purpose storage solution for stream processing systems
//!
//! Britt-Marie offers a set of indexes that are backed by a durable state backend. The
//! implementations are by default lazy. It is however possible to enable COW (Copy on Write) for individual
//! indexes.
//!
//!
//!```text
//!     ValueIndex   HashIndex   ValueIndex
//!          \           |           /
//!           \          |          /
//!            \         |         /
//!             \        |        /
//!              \       |       /
//!             [----RawStore----]
//!```

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

/// BrittMarie Data types
pub mod data;
/// Error types
mod error;
/// Set of compiler hints
mod hint;
/// Available Indexes
mod index;
/// Backing Stores
mod raw_store;

pub use crate::error::BrittMarieError;
pub use crate::index::{
    hash::HashIndex, value::ValueIndex, HashOps, IndexOps, OrderedOps, ValueOps,
};
pub use crate::raw_store::RawStore;

#[cfg(feature = "britt-marie-derive")]
extern crate britt_marie_derive;
#[cfg(feature = "britt-marie-derive")]
#[doc(hidden)]
pub use britt_marie_derive::*;
