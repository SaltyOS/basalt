#![no_std]
#![allow(internal_features)]
#![cfg_attr(feature = "rustc-dep-of-std", feature(no_core, prelude_import))]
#![cfg_attr(feature = "rustc-dep-of-std", no_core)]
#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
#![allow(dead_code, unused_imports)]

#[cfg(feature = "rustc-dep-of-std")]
extern crate rustc_std_workspace_core as core;
#[cfg(all(feature = "std", not(feature = "rustc-dep-of-std")))]
extern crate std as core;

#[cfg(feature = "rustc-dep-of-std")]
#[prelude_import]
use core::prelude::v1::*;

#[cfg(target_os = "saltyos")]
mod saltyos;
#[cfg(target_os = "saltyos")]
pub use saltyos::*;
