//! This module exposes things that are made for the crate to better communicate with an OS.
//!
//! Definition here are public for the case you are dealing with an OS that is not yet
//! supported by default on this crate.

pub use crate::sys::{os::set_os_functions, OsFunctions};

pub mod fd;
