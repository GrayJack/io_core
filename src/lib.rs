#![no_std]
#![allow(incomplete_features, internal_features)]
#![cfg_attr(
    feature = "nightly",
    feature(
        specialization,
        fmt_internals,
        maybe_uninit_array_assume_init,
        maybe_uninit_fill,
        allocator_api,
        never_type,
    )
)]
#![cfg_attr(all(feature = "nightly", feature = "alloc"), feature(try_with_capacity))]
#![cfg_attr(all(doc, feature = "nightly"), feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod io;
pub mod os;

#[doc(hidden)]
pub mod sys;
