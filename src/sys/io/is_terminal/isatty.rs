use core::ffi::c_int;
use std::os::fd::{AsFd, AsRawFd};

unsafe extern "C" {
    unsafe fn isatty(fd: c_int) -> c_int;
}

pub fn is_terminal(fd: &impl AsFd) -> bool {
    let fd = fd.as_fd();
    unsafe { isatty(fd.as_raw_fd()) != 0 }
}
