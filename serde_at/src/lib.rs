//! Serde serializer/deserializer for AT commands

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![allow(deprecated)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_const_for_fn)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

pub mod de;
pub mod ser;

pub use serde;

#[doc(inline)]
pub use self::de::{from_slice, from_str, hex_str::HexStr};
#[doc(inline)]
pub use self::ser::{to_string, to_vec, SerializeOptions};

use core::mem::MaybeUninit;

// TODO: Use `MaybeUninit::uninit_array` once it has stabilized?
fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY: See `MaybeUninit::uninit_array`.
    unsafe {
        #[allow(clippy::uninit_assumed_init)]
        MaybeUninit::uninit().assume_init()
    }
}

// TODO: Use `MaybeUninit::slice_assume_init_ref` once it has stabilized?
unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    // SAFETY: See `MaybeUninit::slice_assume_init_ref`.
    unsafe { &*(slice as *const [MaybeUninit<T>] as *const [T]) }
}
