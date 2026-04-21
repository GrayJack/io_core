use core::fmt;

use crate::io::{
    self, array_buffered::ArrayLineWriterShim, ArrayBufWriter, IntoInnerError, IoSlice, Write,
};

/// Wraps a writer and buffers output to it, flushing whenever a newline
/// (`0x0a`, `'\n'`) is detected.
///
/// The [`BufWriter`] struct wraps a writer and buffers its output.
/// But it only does this batched write when it goes out of scope, or when the
/// internal buffer is full. Sometimes, you'd prefer to write each line as it's
/// completed, rather than the entire buffer at once. Enter `LineWriter`. It
/// does exactly that.
///
/// Like [`BufWriter`], a `LineWriter`’s buffer will also be flushed when the
/// `LineWriter` goes out of scope or when its internal buffer is full.
///
/// If there's still a partial line in the buffer when the `LineWriter` is
/// dropped, it will flush those contents.
///
/// # Examples
///
/// We can use `LineWriter` to write one line at a time, significantly
/// reducing the number of actual writes to the file.
///
/// ```no_run
/// use std::{
///     fs::{self, File},
///     io::{prelude::*, LineWriter},
/// };
///
/// fn main() -> std::io::Result<()> {
///     let road_not_taken = b"I shall be telling this with a sigh
/// Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.";
///
///     let file = File::create("poem.txt")?;
///     let mut file = LineWriter::new(file);
///
///     file.write_all(b"I shall be telling this with a sigh")?;
///
///     // No bytes are written until a newline is encountered (or
///     // the internal buffer is filled).
///     assert_eq!(fs::read_to_string("poem.txt")?, "");
///     file.write_all(b"\n")?;
///     assert_eq!(fs::read_to_string("poem.txt")?, "I shall be telling this with a sigh\n",);
///
///     // Write the rest of the poem.
///     file.write_all(
///         b"Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.",
///     )?;
///
///     // The last line of the poem doesn't end in a newline, so
///     // we have to flush or drop the `LineWriter` to finish
///     // writing.
///     file.flush()?;
///
///     // Confirm the whole poem was written.
///     assert_eq!(fs::read("poem.txt")?, &road_not_taken[..]);
///     Ok(())
/// }
/// ```
pub struct ArrayLineWriter<W: ?Sized + Write, const N: usize> {
    inner: ArrayBufWriter<W, N>,
}

impl<W: Write, const N: usize> ArrayLineWriter<W, N> {
    /// Creates a new `LineWriter`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{fs::File, io::LineWriter};
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let file = LineWriter::new(file);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(inner: W) -> Self {
        Self {
            inner: ArrayBufWriter::new(inner),
        }
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// Caution must be taken when calling methods on the mutable reference
    /// returned as extra writes could corrupt the output stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{fs::File, io::LineWriter};
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let mut file = LineWriter::new(file);
    ///
    ///     // we can use reference just like file
    ///     let reference = file.get_mut();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }

    /// Unwraps this `LineWriter`, returning the underlying writer.
    ///
    /// The internal buffer is written out before returning the writer.
    ///
    /// # Errors
    ///
    /// An [`Err`] will be returned if an error occurs while flushing the buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{fs::File, io::LineWriter};
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///
    ///     let writer: LineWriter<File> = LineWriter::new(file);
    ///
    ///     let file: File = writer.into_inner()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn into_inner(self) -> Result<W, IntoInnerError<ArrayLineWriter<W, N>>> {
        self.inner
            .into_inner()
            .map_err(|err| err.new_wrapped(|inner| ArrayLineWriter { inner }))
    }
}

impl<W: ?Sized + Write, const N: usize> ArrayLineWriter<W, N> {
    /// Gets a reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{fs::File, io::LineWriter};
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::create("poem.txt")?;
    ///     let file = LineWriter::new(file);
    ///
    ///     let reference = file.get_ref();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
}

impl<W: ?Sized + Write, const N: usize> Write for ArrayLineWriter<W, N> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        ArrayLineWriterShim::new(&mut self.inner).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        ArrayLineWriterShim::new(&mut self.inner).write_vectored(bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        ArrayLineWriterShim::new(&mut self.inner).write_all(buf)
    }

    fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        ArrayLineWriterShim::new(&mut self.inner).write_all_vectored(bufs)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        ArrayLineWriterShim::new(&mut self.inner).write_fmt(fmt)
    }
}

impl<W: ?Sized + Write, const N: usize> fmt::Debug for ArrayLineWriter<W, N>
where
    W: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ArrayLineWriter")
            .field("writer", &self.get_ref())
            .field(
                "buffer",
                &format_args!("{}/{}", self.inner.buffer().len(), self.inner.capacity()),
            )
            .finish_non_exhaustive()
    }
}
