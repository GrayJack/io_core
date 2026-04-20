#![allow(unused_imports)]

mod error;
pub use error::RawOsError;

pub use io_slice::{IoSlice, IoSliceMut};
pub use is_terminal::is_terminal;

mod kernel_copy;
pub use kernel_copy::{kernel_copy, CopyState};


// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") { 512 } else { 8 * 1024 };

mod io_slice {
    cfg_select! {
        any(target_family = "unix", target_os = "hermit", target_os = "solid_asp3", target_os = "trusty", target_os = "wasi") => {
            mod iovec;
            pub use iovec::*;
        }
        target_os = "windows" => {
            mod windows;
            pub use windows::*;
        }
        target_os = "uefi" => {
            mod uefi;
            pub use uefi::*;
        }
        _ => {
            mod unsupported;
            pub use unsupported::*;
        }
    }
}

mod is_terminal {
    cfg_select! {
        all(any(target_family = "unix", target_os = "wasi"), feature = "std") => {
            mod isatty;
            pub use isatty::*;
        }
        all(target_os = "windows", feature = "std") => {
            mod windows;
            pub use windows::*;
        }
        all(target_os = "hermit", feature = "std") => {
            mod hermit;
            pub use hermit::*;
        }
        all(target_os = "motor", feature = "std") => {
            mod motor;
            pub use motor::*;
        }
        _ => {
            mod unsupported;
            pub use unsupported::*;
        }
    }
}
