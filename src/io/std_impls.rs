//! Implementation of std traits for types of this crate and implementation of this crate trait for
//! std types
#![allow(clippy::missing_transmute_annotations)]

use core::mem;

use crate::io;

macro_rules! std_read {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ::std::io::Read for $t {
                std_read!();
            }
        )+
    };

    () => {
        fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
            <Self as io::Read>::read(self, buf).map_err(Into::into)
        }

        fn read_vectored(&mut self, bufs: &mut [::std::io::IoSliceMut<'_>]) -> ::std::io::Result<usize> {
            let bufs = unsafe { mem::transmute(bufs) };
            <Self as io::Read>::read_vectored(self, bufs).map_err(Into::into)
        }

        fn read_to_end(&mut self, buf: &mut std::vec::Vec<u8>) -> ::std::io::Result<usize> {
            <Self as io::Read>::read_to_end(self, buf).map_err(Into::into)
        }

        fn read_to_string(&mut self, buf: &mut std::string::String) -> ::std::io::Result<usize> {
            <Self as io::Read>::read_to_string(self, buf).map_err(Into::into)
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> ::std::io::Result<()> {
            <Self as io::Read>::read_exact(self, buf).map_err(Into::into)
        }
    };
}

macro_rules! std_bufread {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ::std::io::BufRead for $t {
                std_bufread!();
            }
        )+
    };

    () => {
        fn fill_buf(&mut self) -> ::std::io::Result<&[u8]> {
            <Self as io::BufRead>::fill_buf(self).map_err(Into::into)
        }

        fn consume(&mut self, amount: usize) {
            <Self as io::BufRead>::consume(self, amount)
        }

        fn read_until(&mut self, byte: u8, buf: &mut std::vec::Vec<u8>) -> ::std::io::Result<usize> {
            <Self as io::BufRead>::read_until(self, byte, buf).map_err(Into::into)
        }

        fn skip_until(&mut self, byte: u8) -> ::std::io::Result<usize> {
            <Self as io::BufRead>::skip_until(self, byte).map_err(Into::into)
        }

        fn read_line(&mut self, buf: &mut std::string::String) -> ::std::io::Result<usize> {
            <Self as io::BufRead>::read_line(self, buf).map_err(Into::into)
        }
    };
}

macro_rules! std_write {
    ($($t:ty),+ $(,)?) => {
        $(
            impl ::std::io::Write for $t {
                std_write!();
            }
        )+
    };

    () => {
        fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize> {
            <Self as io::Write>::write(self, buf).map_err(Into::into)
        }

        fn flush(&mut self) -> ::std::io::Result<()> {
            <Self as io::Write>::flush(self).map_err(Into::into)
        }

        fn write_vectored(&mut self, bufs: &[::std::io::IoSlice<'_>]) -> ::std::io::Result<usize> {
            let bufs = unsafe { mem::transmute(bufs) };
            <Self as io::Write>::write_vectored(self, bufs).map_err(Into::into)
        }

        fn write_all(&mut self, mut buf: &[u8]) -> ::std::io::Result<()> {
            <Self as io::Write>::write_all(self, buf).map_err(Into::into)
        }

        fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> ::std::io::Result<()> {
            <Self as io::Write>::write_fmt(self, args).map_err(Into::into)
        }
    };
}

macro_rules! crate_read {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::io::Read for $t {
               crate_read!();
            }
        )+
    };

    () => {
        fn read(&mut self, buf: &mut [u8]) -> $crate::io::Result<usize> {
            <Self as ::std::io::Read>::read(self, buf).map_err(Into::into)
        }

        fn read_vectored(&mut self, bufs: &mut [$crate::io::IoSliceMut<'_>]) -> $crate::io::Result<usize> {
            let bufs = unsafe { mem::transmute(bufs) };
            <Self as ::std::io::Read>::read_vectored(self, bufs).map_err(Into::into)
        }

        fn read_to_end(&mut self, buf: &mut std::vec::Vec<u8>) -> $crate::io::Result<usize> {
            <Self as ::std::io::Read>::read_to_end(self, buf).map_err(Into::into)
        }

        fn read_to_string(&mut self, buf: &mut std::string::String) -> $crate::io::Result<usize> {
            <Self as ::std::io::Read>::read_to_string(self, buf).map_err(Into::into)
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> $crate::io::Result<()> {
            <Self as ::std::io::Read>::read_exact(self, buf).map_err(Into::into)
        }
    };
}

macro_rules! crate_bufread {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::io::BufRead for $t {
                crate_bufread!();
            }
        )+
    };

    () => {
        fn fill_buf(&mut self) -> $crate::io::Result<&[u8]> {
            <Self as ::std::io::BufRead>::fill_buf(self).map_err(Into::into)
        }

        fn consume(&mut self, amount: usize) {
            <Self as ::std::io::BufRead>::consume(self, amount)
        }

        fn read_until(&mut self, byte: u8, buf: &mut std::vec::Vec<u8>) -> $crate::io::Result<usize> {
            <Self as ::std::io::BufRead>::read_until(self, byte, buf).map_err(Into::into)
        }

        fn skip_until(&mut self, byte: u8) -> $crate::io::Result<usize> {
            <Self as ::std::io::BufRead>::skip_until(self, byte).map_err(Into::into)
        }

        fn read_line(&mut self, buf: &mut std::string::String) -> $crate::io::Result<usize> {
            <Self as ::std::io::BufRead>::read_line(self, buf).map_err(Into::into)
        }
    }
}

macro_rules! crate_write {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::io::Write for $t {
                crate_write!();
            }
        )+
    };

    () => {
        fn write(&mut self, buf: &[u8]) -> $crate::io::Result<usize> {
            <Self as ::std::io::Write>::write(self, buf).map_err(Into::into)
        }

        fn flush(&mut self) -> $crate::io::Result<()> {
            <Self as ::std::io::Write>::flush(self).map_err(Into::into)
        }

        fn write_vectored(&mut self, bufs: &[$crate::io::IoSlice<'_>]) -> $crate::io::Result<usize> {
            let bufs = unsafe { mem::transmute(bufs) };
            <Self as ::std::io::Write>::write_vectored(self, bufs).map_err(Into::into)
        }

        fn write_all(&mut self, mut buf: &[u8]) -> $crate::io::Result<()> {
            <Self as ::std::io::Write>::write_all(self, buf).map_err(Into::into)
        }

        fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> $crate::io::Result<()> {
            <Self as ::std::io::Write>::write_fmt(self, args).map_err(Into::into)
        }
    };
}

std_read!(io::Empty, io::Repeat);
std_bufread!(io::Empty);
std_write!(
    io::Empty,
    &io::Empty,
    io::Sink,
    &io::Sink,
    io::BorrowedCursor<'_>,
    io::TrackingBorrowedCursor<'_>,
    io::PipeWriter,
    &io::PipeWriter,
);

crate_read!(
    std::io::Empty,
    std::io::Repeat,
    std::io::Stdin,
    &std::io::Stdin,
    std::io::StdinLock<'_>,
    std::io::PipeReader,
    &std::io::PipeReader,
    std::fs::File,
    &std::fs::File,
    std::sync::Arc<std::fs::File>,
    std::net::TcpStream,
    &std::net::TcpStream,
    std::process::ChildStdout,
    std::process::ChildStderr,
);
crate_bufread!(std::io::Empty, std::io::StdinLock<'_>,);
crate_write!(
    std::io::Empty,
    &std::io::Empty,
    std::io::Sink,
    &std::io::Sink,
    std::io::Stderr,
    &std::io::Stderr,
    std::io::StderrLock<'_>,
    std::io::Stdout,
    &std::io::Stdout,
    std::io::StdoutLock<'_>,
    std::io::PipeWriter,
    &std::io::PipeWriter,
    std::fs::File,
    &std::fs::File,
    std::net::TcpStream,
    &std::net::TcpStream,
    std::process::ChildStdin,
    &std::process::ChildStdin,
);


impl<R: ?Sized + io::Read> std::io::Read for io::BufReader<R> {
    std_read!();
}

impl<R: ?Sized + io::Read> std::io::BufRead for io::BufReader<R> {
    std_bufread!();
}

impl<W: ?Sized + io::Write> std::io::Write for io::BufWriter<W> {
    std_write!();
}

impl<W: ?Sized + io::Write> std::io::Write for io::LineWriter<W> {
    std_write!();
}

impl<R: ?Sized + io::Read, const N: usize> std::io::Read for io::ArrayBufReader<R, N> {
    std_read!();
}

impl<R: ?Sized + io::Read, const N: usize> std::io::BufRead for io::ArrayBufReader<R, N> {
    std_bufread!();
}

impl<W: ?Sized + io::Write, const N: usize> std::io::Write for io::ArrayBufWriter<W, N> {
    std_write!();
}

impl<W: ?Sized + io::Write, const N: usize> std::io::Write for io::ArrayLineWriter<W, N> {
    std_write!();
}

impl<R: io::Read> std::io::Read for io::Take<R> {
    std_read!();
}

impl<T: io::Read, U: io::Read> std::io::Read for io::Chain<T, U> {
    std_read!();
}

impl<R: ?Sized + std::io::Read> io::Read for std::io::BufReader<R> {
    crate_read!();
}

impl<R: ?Sized + std::io::Read> io::BufRead for std::io::BufReader<R> {
    crate_bufread!();
}

impl<W: ?Sized + std::io::Write> io::Write for std::io::BufWriter<W> {
    crate_write!();
}

impl<W: ?Sized + std::io::Write> io::Write for std::io::LineWriter<W> {
    crate_write!();
}

impl<T: AsRef<[u8]>> io::Read for std::io::Cursor<T> {
    crate_read!();
}

impl<T: AsRef<[u8]>> io::BufRead for std::io::Cursor<T> {
    crate_bufread!();
}

impl<R: std::io::Read> io::Read for std::io::Take<R> {
    crate_read!();
}

impl<R: std::io::BufRead> io::BufRead for std::io::Take<R> {
    crate_bufread!();
}

impl<T: std::io::Read, U: std::io::Read> io::Read for std::io::Chain<T, U> {
    crate_read!();
}

impl<T: std::io::BufRead, U: std::io::BufRead> io::BufRead for std::io::Chain<T, U> {
    crate_bufread!();
}

impl From<io::Error> for std::io::Error {
    fn from(value: io::Error) -> Self {
        // Safety: We copied the code from the std and we represent the error the exactly same way
        // when this crate is compiled with `feature = "std"``.
        unsafe { core::mem::transmute(value) }
    }
}

impl From<std::io::Error> for io::Error {
    fn from(value: std::io::Error) -> Self {
        // Safety: We copied the code from the std and we represent the error the exactly same way
        // when this crate is compiled with `feature = "std"``.
        unsafe { core::mem::transmute(value) }
    }
}

impl From<std::io::IoSlice<'_>> for io::IoSlice<'_> {
    fn from(value: std::io::IoSlice<'_>) -> Self {
        // Safety: Same type and same layout as std
        unsafe { mem::transmute(value) }
    }
}

impl From<io::IoSlice<'_>> for std::io::IoSlice<'_> {
    fn from(value: io::IoSlice<'_>) -> Self {
        // Safety: Same type and same layout as std
        unsafe { mem::transmute(value) }
    }
}

impl From<std::io::IoSliceMut<'_>> for io::IoSliceMut<'_> {
    fn from(value: std::io::IoSliceMut<'_>) -> Self {
        // Safety: Same type and same layout as std
        unsafe { mem::transmute(value) }
    }
}

impl From<io::IoSliceMut<'_>> for std::io::IoSliceMut<'_> {
    fn from(value: io::IoSliceMut<'_>) -> Self {
        // Safety: Same type and same layout as std
        unsafe { mem::transmute(value) }
    }
}
