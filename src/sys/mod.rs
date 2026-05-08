#![allow(dead_code)]

pub mod io;
pub mod pipe;

#[cfg(feature = "alloc")]
use alloc::string::String;

use crate::{io::ErrorKind, sys::io::RawOsError};

/// A V-table with the functions needed for some features that are OS-dependent
#[derive(Debug)]
pub struct OsFunctions {
    #[cfg(feature = "alloc")]
    pub error_string: fn(_: RawOsError) -> String,
    pub error_str: fn(_: RawOsError) -> &'static str,
    pub decode_error_kind: fn(_: RawOsError) -> ErrorKind,
    pub is_interrupted: fn(_: RawOsError) -> bool,
    pub last_os_error: fn() -> RawOsError,
}

#[cfg(target_has_atomic = "ptr")]
pub(crate) mod os {
    use core::sync::atomic::{AtomicPtr, Ordering};

    #[cfg(feature = "alloc")]
    use alloc::string::String;

    use crate::{io::ErrorKind, os::OsFunctions, sys::io::RawOsError};

    pub(crate) const DEFAULT_OS_FUNCTIONS: &OsFunctions = &OsFunctions {
        error_str: |_| "",
        #[cfg(feature = "alloc")]
        error_string: |_| String::new(),
        decode_error_kind: |_| ErrorKind::Uncategorized,
        is_interrupted: |_| false,
        last_os_error: || 0,
    };

    pub(crate) const UNIX_OS_FUNCTIONS: &OsFunctions = &OsFunctions {
        error_str: |_| "",
        #[cfg(feature = "alloc")]
        error_string: super::io::error_string,
        decode_error_kind: super::io::decode_error_kind,
        is_interrupted: super::io::is_interrupted,
        last_os_error: super::io::errno,
    };

    pub(crate) static OS_FUNCTIONS: AtomicPtr<OsFunctions> = cfg_select! {
        target_family = "unix" => AtomicPtr::new(UNIX_OS_FUNCTIONS as *const _ as *mut _),
        _ => AtomicPtr::new(DEFAULT_OS_FUNCTIONS as *const _ as *mut _),
    };

    /// Sets the functions used by this crate to have expandable OS handling.
    ///
    /// This should be called before any I/O interaction to be properly used by this crate if you
    /// are in a system we have no support by default.
    ///
    /// This function allows you to have robust I/O Error support even on your custom system or
    /// niche OS.
    #[inline]
    pub fn set_os_functions(f: &'static OsFunctions) {
        OS_FUNCTIONS.store(f as *const _ as *mut _, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn error_str(errno: RawOsError) -> &'static str {
        let f = unsafe { &*OS_FUNCTIONS.load(Ordering::Relaxed) };
        (f.error_str)(errno)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub(crate) fn error_string(errno: RawOsError) -> String {
        let f = unsafe { &*OS_FUNCTIONS.load(Ordering::Relaxed) };
        (f.error_string)(errno)
    }

    #[inline]
    pub(crate) fn decode_error_kind(errno: RawOsError) -> ErrorKind {
        let f = unsafe { &*OS_FUNCTIONS.load(Ordering::Relaxed) };
        (f.decode_error_kind)(errno)
    }

    #[inline]
    pub(crate) fn is_interrupted(errno: RawOsError) -> bool {
        let f = unsafe { &*OS_FUNCTIONS.load(Ordering::Relaxed) };
        (f.is_interrupted)(errno)
    }

    #[inline]
    pub(crate) fn last_os_error() -> RawOsError {
        let f = unsafe { &*OS_FUNCTIONS.load(Ordering::Relaxed) };
        (f.last_os_error)()
    }
}

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) mod os {
    #[cfg(feature = "alloc")]
    use alloc::string::String;

    use crate::{io::ErrorKind, sys::io::RawOsError, OsFunctions};

    /// # Warning
    ///
    /// You are out of luck! This system doesn't support `AtomicPtr`, so this function is a no_op.
    ///
    /// This is not my top priority as of now, but if many people hits this, we can work together on
    /// a solution that works for system without `AtomicPtr``.
    ///
    /// # About the function
    ///
    /// Sets the functions used by this crate to have expandable OS handling.
    ///
    /// This should be called before any I/O interaction to be properly used by this crate if you
    /// are in a system we have no support by default.
    ///
    /// This function allows you to have robust I/O Error support even on your custom system or
    /// niche OS.
    #[inline]
    pub fn set_os_functions(_f: &'static OsFunctions) {}

    #[inline]
    pub(crate) fn error_str(errno: RawOsError) -> &'static str {
        ""
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub(crate) fn error_string(errno: RawOsError) -> String {
        String::new()
    }

    #[inline]
    pub(crate) fn decode_error_kind(errno: RawOsError) -> ErrorKind {
        ErrorKind::Uncategorized
    }

    #[inline]
    pub(crate) fn is_interrupted(errno: RawOsError) -> bool {
        false
    }

    #[inline]
    pub(crate) fn last_os_error() -> RawOsError {
        0
    }
}


/// A trait for viewing representations from std types.
#[cfg_attr(not(target_os = "linux"), allow(unused))]
pub trait AsInner<Inner: ?Sized> {
    fn as_inner(&self) -> &Inner;
}

/// A trait for viewing representations from std types.
#[cfg_attr(not(target_os = "linux"), allow(unused))]
pub trait AsInnerMut<Inner: ?Sized> {
    fn as_inner_mut(&mut self) -> &mut Inner;
}

/// A trait for extracting representations from std types.
pub trait IntoInner<Inner> {
    fn into_inner(self) -> Inner;
}

/// A trait for creating std types from internal representations.
pub trait FromInner<Inner> {
    fn from_inner(inner: Inner) -> Self;
}
