#![allow(unused)]
use core::fmt;


#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};


enum Never {}

#[cfg(feature = "nightly")]
pub struct Pipe(!);
#[cfg(not(feature = "nightly"))]
pub struct Pipe(Never);

#[inline]
pub fn pipe() -> io::Result<(Pipe, Pipe)> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}

impl Pipe {
    pub fn try_clone(&self) -> io::Result<Self> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn read(&self, _buf: &mut [u8]) -> io::Result<usize> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn read_buf(&self, _buf: BorrowedCursor<'_>) -> io::Result<()> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn read_vectored(&self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn is_read_vectored(&self) -> bool {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    #[cfg(feature = "alloc")]
    pub fn read_to_end(&self, _buf: &mut Vec<u8>) -> io::Result<usize> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn write(&self, _buf: &[u8]) -> io::Result<usize> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn write_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn is_write_vectored(&self) -> bool {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }

    pub fn diverge(&self) -> ! {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }
}

impl fmt::Debug for Pipe {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        cfg_select! {
            feature = "nightly" => self.0,
            _ => todo!()
        }
    }
}

#[cfg(all(any(unix, target_os = "hermit", target_os = "wasi"), feature = "std"))]
mod unix_traits {

    use super::Pipe;
    use crate::sys::{FromInner, IntoInner};
    use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};

    impl AsRawFd for Pipe {
        #[inline]
        fn as_raw_fd(&self) -> RawFd {
            cfg_select! {
                feature = "nightly" => self.0,
                _ => todo!()
            }
        }
    }

    impl AsFd for Pipe {
        fn as_fd(&self) -> BorrowedFd<'_> {
            cfg_select! {
                feature = "nightly" => self.0,
                _ => todo!()
            }
        }
    }

    impl IntoRawFd for Pipe {
        fn into_raw_fd(self) -> RawFd {
            cfg_select! {
                feature = "nightly" => self.0,
                _ => todo!()
            }
        }
    }

    impl FromRawFd for Pipe {
        unsafe fn from_raw_fd(_: RawFd) -> Self {
            panic!("creating pipe on this platform is unsupported!")
        }
    }

    impl FromInner<OwnedFd> for Pipe {
        fn from_inner(_: OwnedFd) -> Self {
            panic!("creating pipe on this platform is unsupported!")
        }
    }

    impl IntoInner<OwnedFd> for Pipe {
        fn into_inner(self) -> OwnedFd {
            cfg_select! {
                feature = "nightly" => self.0,
                _ => todo!()
            }
        }
    }
}
