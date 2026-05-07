use std::{fs, io, os::fd};

use crate::os::fd::RawFd;

impl super::AsRawFd for io::PipeReader {
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::FromRawFd for io::PipeReader {
    unsafe fn from_raw_fd(raw_fd: RawFd) -> Self {
        unsafe { <Self as fd::FromRawFd>::from_raw_fd(raw_fd) }
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::IntoRawFd for io::PipeReader {
    fn into_raw_fd(self) -> RawFd {
        <Self as fd::IntoRawFd>::into_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::AsRawFd for io::PipeWriter {
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::FromRawFd for io::PipeWriter {
    unsafe fn from_raw_fd(raw_fd: RawFd) -> Self {
        unsafe { <Self as fd::FromRawFd>::from_raw_fd(raw_fd) }
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::IntoRawFd for io::PipeWriter {
    fn into_raw_fd(self) -> RawFd {
        <Self as fd::IntoRawFd>::into_raw_fd(self)
    }
}

impl super::AsRawFd for fs::File {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::FromRawFd for fs::File {
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> fs::File {
        unsafe { <Self as fd::FromRawFd>::from_raw_fd(fd) }
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::IntoRawFd for fs::File {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        <Self as fd::IntoRawFd>::into_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl super::AsRawFd for io::Stdin {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

impl super::AsRawFd for io::Stdout {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

impl super::AsRawFd for io::Stderr {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

#[cfg(not(target_os = "trusty"))]
impl<'a> super::AsRawFd for io::StdinLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

impl<'a> super::AsRawFd for io::StdoutLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}

impl<'a> super::AsRawFd for io::StderrLock<'a> {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        <Self as fd::AsRawFd>::as_raw_fd(self)
    }
}
