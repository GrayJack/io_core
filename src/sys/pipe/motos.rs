use crate::{io, sys::fd::FileDesc};

pub type Pipe = FileDesc;

#[inline]
pub fn pipe() -> io::Result<(Pipe, Pipe)> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}
