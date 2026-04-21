use core::{error, fmt, result};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::sys;

// On 64-bit platforms, `io::Error` may use a bit-packed representation to
// reduce size. However, this representation assumes that error codes are
// always 32-bit wide.
//
// This assumption is invalid on 64-bit UEFI, where error codes are 64-bit.
// Therefore, the packed representation is explicitly disabled for UEFI
// targets, and the unpacked representation must be used instead.
#[cfg(all(target_pointer_width = "64", not(target_os = "uefi"), feature = "alloc"))]
mod repr_bitpacked;
#[cfg(all(target_pointer_width = "64", not(target_os = "uefi"), feature = "alloc"))]
use repr_bitpacked::Repr;

#[cfg(any(not(target_pointer_width = "64"), target_os = "uefi", not(feature = "alloc")))]
mod repr_unpacked;
#[cfg(any(not(target_pointer_width = "64"), target_os = "uefi", not(feature = "alloc")))]
use repr_unpacked::Repr;

/// A specialized [`Result`] type for I/O operations.
///
/// This type is broadly used across [`std::io`] for any operation which may
/// produce an error.
///
/// This type alias is generally used to avoid writing out [`io::Error`] directly and
/// is otherwise a direct mapping to [`Result`].
///
/// While usual Rust style is to import types directly, aliases of [`Result`]
/// often are not, to make it easier to distinguish between them. [`Result`] is
/// generally assumed to be [`std::result::Result`][`Result`], and so users of this alias
/// will generally use `io::Result` instead of shadowing the [prelude]'s import
/// of [`std::result::Result`][`Result`].
///
/// [`std::io`]: crate::io
/// [`io::Error`]: Error
/// [`Result`]: crate::result::Result
/// [prelude]: crate::prelude
///
/// # Examples
///
/// A convenience function that bubbles an `io::Result` to its caller:
///
/// ```
/// use std::io;
///
/// fn get_string() -> io::Result<String> {
///     let mut buffer = String::new();
///
///     io::stdin().read_line(&mut buffer)?;
///
///     Ok(buffer)
/// }
/// ```
pub type Result<T> = result::Result<T, Error>;

/// The error type for I/O operations of the [`Read`], [`Write`], [`Seek`], and
/// associated traits.
///
/// Errors mostly originate from the underlying OS, but custom instances of
/// `Error` can be created with crafted error messages and a particular value of
/// [`ErrorKind`].
///
/// [`Read`]: crate::io::Read
/// [`Write`]: crate::io::Write
/// [`Seek`]: crate::io::Seek
pub struct Error {
    repr: Repr,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.repr, f)
    }
}

/// Common errors constants for use in std
#[allow(dead_code)]
impl Error {
    pub(crate) const INVALID_UTF8: Self =
        crate::const_error!(ErrorKind::InvalidData, "stream did not contain valid UTF-8");
    pub(crate) const NO_ADDRESSES: Self =
        crate::const_error!(ErrorKind::InvalidInput, "could not resolve to any addresses");
    pub(crate) const READ_EXACT_EOF: Self =
        crate::const_error!(ErrorKind::UnexpectedEof, "failed to fill whole buffer");
    pub(crate) const UNKNOWN_THREAD_COUNT: Self = crate::const_error!(
        ErrorKind::NotFound,
        "the number of hardware threads is not known for the target platform",
    );
    pub(crate) const UNSUPPORTED_PLATFORM: Self =
        crate::const_error!(ErrorKind::Unsupported, "operation not supported on this platform");
    pub(crate) const WRITE_ALL_EOF: Self =
        crate::const_error!(ErrorKind::WriteZero, "failed to write whole buffer");
    pub(crate) const ZERO_TIMEOUT: Self =
        crate::const_error!(ErrorKind::InvalidInput, "cannot set a 0 duration timeout");
}

#[cfg(feature = "alloc")]
impl From<alloc::ffi::NulError> for Error {
    /// Converts a [`alloc::ffi::NulError`] into a [`Error`].
    fn from(_: alloc::ffi::NulError) -> Error {
        crate::const_error!(ErrorKind::InvalidInput, "data provided contains a nul byte")
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::collections::TryReserveError> for Error {
    /// Converts `TryReserveError` to an error with [`ErrorKind::OutOfMemory`].
    ///
    /// `TryReserveError` won't be available as the error `source()`,
    /// but this may change in the future.
    fn from(_: alloc::collections::TryReserveError) -> Error {
        // ErrorData::Custom allocates, which isn't great for handling OOM errors.
        ErrorKind::OutOfMemory.into()
    }
}

// Only derive debug in tests, to make sure it
// doesn't accidentally get printed.
#[cfg_attr(test, derive(Debug))]
enum ErrorData<C> {
    Os(RawOsError),
    Simple(ErrorKind),
    SimpleMessage(&'static SimpleMessage),
    Custom(C),
}

/// The type of raw OS error codes returned by [`Error::raw_os_error`].
///
/// This is an [`i32`] on all currently supported platforms, but platforms
/// added in the future (such as UEFI) may use a different primitive type like
/// [`usize`]. Use `as`or [`into`] conversions where applicable to ensure maximum
/// portability.
///
/// [`into`]: Into::into
pub type RawOsError = sys::io::RawOsError;

// `#[repr(align(4))]` is probably redundant, it should have that value or
// higher already. We include it just because repr_bitpacked.rs's encoding
// requires an alignment >= 4 (note that `#[repr(align)]` will not reduce the
// alignment required by the struct, only increase it).
//
// If we add more variants to ErrorData, this can be increased to 8, but it
// should probably be behind `#[cfg_attr(target_pointer_width = "64", ...)]` or
// whatever cfg we're using to enable the `repr_bitpacked` code, since only the
// that version needs the alignment, and 8 is higher than the alignment we'll
// have on 32 bit platforms.
//
// (For the sake of being explicit: the alignment requirement here only matters
// if `error/repr_bitpacked.rs` is in use — for the unpacked repr it doesn't
// matter at all)
#[doc(hidden)]
#[repr(align(4))]
#[derive(Debug)]
pub struct SimpleMessage {
    pub kind: ErrorKind,
    pub message: &'static str,
}

/// Creates a new I/O error from a known kind of error and a string literal.
///
/// Contrary to [`Error::new`], this macro does not allocate and can be used in
/// `const` contexts.
///
/// # Example
/// ```
/// #![feature(io_const_error)]
/// use std::io::{const_error, Error, ErrorKind};
///
/// const FAIL: Error = const_error!(ErrorKind::Unsupported, "tried something that never works");
///
/// fn not_here() -> Result<(), Error> {
///     Err(FAIL)
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! const_error {
    ($kind:expr, $message:expr $(,)?) => {
        $crate::io::Error::from_static_message(
            const {
                &$crate::io::SimpleMessage {
                    kind: $kind,
                    message: $message,
                }
            },
        )
    };
}

// As with `SimpleMessage`: `#[repr(align(4))]` here is just because
// repr_bitpacked's encoding requires it. In practice it almost certainly be
// already be this high or higher.
#[derive(Debug)]
#[repr(align(4))]
struct Custom {
    kind: ErrorKind,
    #[cfg(feature = "alloc")]
    error: Box<dyn error::Error + Send + Sync>,
}

/// A list specifying general categories of I/O error.
///
/// This list is intended to grow over time and it is not recommended to
/// exhaustively match against it.
///
/// It is used with the [`io::Error`] type.
///
/// [`io::Error`]: Error
///
/// # Handling errors and matching on `ErrorKind`
///
/// In application code, use `match` for the `ErrorKind` values you are
/// expecting; use `_` to match "all other errors".
///
/// In comprehensive and thorough tests that want to verify that a test doesn't
/// return any known incorrect error kind, you may want to cut-and-paste the
/// current full list of errors from here into your test code, and then match
/// `_` as the correct case. This seems counterintuitive, but it will make your
/// tests more robust. In particular, if you want to verify that your code does
/// produce an unrecognized error kind, the robust solution is to check for all
/// the recognized error kinds and fail in those cases.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[allow(deprecated)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An entity was not found, often a file.
    NotFound,
    /// The operation lacked the necessary privileges to complete.
    PermissionDenied,
    /// The connection was refused by the remote server.
    ConnectionRefused,
    /// The connection was reset by the remote server.
    ConnectionReset,
    /// The remote host is not reachable.
    HostUnreachable,
    /// The network containing the remote host is not reachable.
    NetworkUnreachable,
    /// The connection was aborted (terminated) by the remote server.
    ConnectionAborted,
    /// The network operation failed because it was not connected yet.
    NotConnected,
    /// A socket address could not be bound because the address is already in
    /// use elsewhere.
    AddrInUse,
    /// A nonexistent interface was requested or the requested address was not
    /// local.
    AddrNotAvailable,
    /// The system's networking is down.
    NetworkDown,
    /// The operation failed because a pipe was closed.
    BrokenPipe,
    /// An entity already exists, often a file.
    AlreadyExists,
    /// The operation needs to block to complete, but the blocking operation was
    /// requested to not occur.
    WouldBlock,
    /// A filesystem object is, unexpectedly, not a directory.
    ///
    /// For example, a filesystem path was specified where one of the intermediate directory
    /// components was, in fact, a plain file.
    NotADirectory,
    /// The filesystem object is, unexpectedly, a directory.
    ///
    /// A directory was specified when a non-directory was expected.
    IsADirectory,
    /// A non-empty directory was specified where an empty directory was expected.
    DirectoryNotEmpty,
    /// The filesystem or storage medium is read-only, but a write operation was attempted.
    ReadOnlyFilesystem,
    /// Loop in the filesystem or IO subsystem; often, too many levels of symbolic links.
    ///
    /// There was a loop (or excessively long chain) resolving a filesystem object
    /// or file IO object.
    ///
    /// On Unix this is usually the result of a symbolic link loop; or, of exceeding the
    /// system-specific limit on the depth of symlink traversal.
    FilesystemLoop,
    /// Stale network file handle.
    ///
    /// With some network filesystems, notably NFS, an open file (or directory) can be invalidated
    /// by problems with the network or server.
    StaleNetworkFileHandle,
    /// A parameter was incorrect.
    InvalidInput,
    /// Data not valid for the operation were encountered.
    ///
    /// Unlike [`InvalidInput`], this typically means that the operation
    /// parameters were valid, however the error was caused by malformed
    /// input data.
    ///
    /// For example, a function that reads a file into a string will error with
    /// `InvalidData` if the file's contents are not valid UTF-8.
    ///
    /// [`InvalidInput`]: ErrorKind::InvalidInput
    InvalidData,
    /// The I/O operation's timeout expired, causing it to be canceled.
    TimedOut,
    /// An error returned when an operation could not be completed because a
    /// call to [`write`] returned [`Ok(0)`].
    ///
    /// This typically means that an operation could only succeed if it wrote a
    /// particular number of bytes but only a smaller number of bytes could be
    /// written.
    ///
    /// [`write`]: crate::io::Write::write
    /// [`Ok(0)`]: Ok
    WriteZero,
    /// The underlying storage (typically, a filesystem) is full.
    ///
    /// This does not include out of quota errors.
    StorageFull,
    /// Seek on unseekable file.
    ///
    /// Seeking was attempted on an open file handle which is not suitable for seeking - for
    /// example, on Unix, a named pipe opened with `File::open`.
    NotSeekable,
    /// Filesystem quota or some other kind of quota was exceeded.
    QuotaExceeded,
    /// File larger than allowed or supported.
    ///
    /// This might arise from a hard limit of the underlying filesystem or file access API, or from
    /// an administratively imposed resource limitation.  Simple disk full, and out of quota, have
    /// their own errors.
    FileTooLarge,
    /// Resource is busy.
    ResourceBusy,
    /// Executable file is busy.
    ///
    /// An attempt was made to write to a file which is also in use as a running program.  (Not all
    /// operating systems detect this situation.)
    ExecutableFileBusy,
    /// Deadlock (avoided).
    ///
    /// A file locking operation would result in deadlock.  This situation is typically detected,
    /// if at all, on a best-effort basis.
    Deadlock,
    /// Cross-device or cross-filesystem (hard) link or rename.
    CrossesDevices,
    /// Too many (hard) links to the same filesystem object.
    ///
    /// The filesystem does not support making so many hardlinks to the same file.
    TooManyLinks,
    /// A filename was invalid.
    ///
    /// This error can also occur if a length limit for a name was exceeded.
    InvalidFilename,
    /// Program argument list too long.
    ///
    /// When trying to run an external program, a system or process limit on the size of the
    /// arguments would have been exceeded.
    ArgumentListTooLong,
    /// This operation was interrupted.
    ///
    /// Interrupted operations can typically be retried.
    Interrupted,

    /// This operation is unsupported on this platform.
    ///
    /// This means that the operation can never succeed.
    Unsupported,

    // ErrorKinds which are primarily categorisations for OS error
    // codes should be added above.
    /// An error returned when an operation could not be completed because an
    /// "end of file" was reached prematurely.
    ///
    /// This typically means that an operation could only succeed if it read a
    /// particular number of bytes but only a smaller number of bytes could be
    /// read.
    UnexpectedEof,

    /// An operation could not be completed, because it failed
    /// to allocate enough memory.
    OutOfMemory,

    /// The operation was partially successful and needs to be checked
    /// later on due to not blocking.
    InProgress,

    // "Unusual" error kinds which do not correspond simply to (sets
    // of) OS error codes, should be added just above this comment.
    // `Other` and `Uncategorized` should remain at the end:
    /// A custom error that does not fall under any other I/O error kind.
    ///
    /// This can be used to construct your own [`Error`]s that do not match any
    /// [`ErrorKind`].
    ///
    /// This [`ErrorKind`] is not used by the standard library.
    ///
    /// Errors from the standard library that do not fall under any of the I/O
    /// error kinds cannot be `match`ed on, and will only match a wildcard (`_`) pattern.
    /// New [`ErrorKind`]s might be added in the future for some of those.
    Other,

    /// Any I/O error from the standard library that's not part of this list.
    ///
    /// Errors that are `Uncategorized` now may move to a different or a new
    /// [`ErrorKind`] variant in the future. It is not recommended to match
    /// an error against `Uncategorized`; use a wildcard match (`_`) instead.
    #[doc(hidden)]
    Uncategorized,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        match *self {
            // tidy-alphabetical-start
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            AlreadyExists => "entity already exists",
            ArgumentListTooLong => "argument list too long",
            BrokenPipe => "broken pipe",
            ConnectionAborted => "connection aborted",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            CrossesDevices => "cross-device link or rename",
            Deadlock => "deadlock",
            DirectoryNotEmpty => "directory not empty",
            ExecutableFileBusy => "executable file busy",
            FileTooLarge => "file too large",
            FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            HostUnreachable => "host unreachable",
            InProgress => "in progress",
            Interrupted => "operation interrupted",
            InvalidData => "invalid data",
            InvalidFilename => "invalid filename",
            InvalidInput => "invalid input parameter",
            IsADirectory => "is a directory",
            NetworkDown => "network down",
            NetworkUnreachable => "network unreachable",
            NotADirectory => "not a directory",
            NotConnected => "not connected",
            NotFound => "entity not found",
            NotSeekable => "seek on unseekable file",
            Other => "other error",
            OutOfMemory => "out of memory",
            PermissionDenied => "permission denied",
            QuotaExceeded => "quota exceeded",
            ReadOnlyFilesystem => "read-only filesystem or storage medium",
            ResourceBusy => "resource busy",
            StaleNetworkFileHandle => "stale network file handle",
            StorageFull => "no storage space",
            TimedOut => "timed out",
            TooManyLinks => "too many links",
            Uncategorized => "uncategorized error",
            UnexpectedEof => "unexpected end of file",
            Unsupported => "unsupported",
            WouldBlock => "operation would block",
            WriteZero => "write zero",
            // tidy-alphabetical-end
        }
    }
}

impl fmt::Display for ErrorKind {
    /// Shows a human-readable description of the `ErrorKind`.
    ///
    /// This is similar to `impl Display for Error`, but doesn't require first converting to Error.
    ///
    /// # Examples
    /// ```
    /// use std::io::ErrorKind;
    /// assert_eq!("entity not found", ErrorKind::NotFound.to_string());
    /// ```
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.as_str())
    }
}

/// Intended for use for errors not exposed to the user, where allocating onto
/// the heap (for normal construction via Error::new) is too costly.
impl From<ErrorKind> for Error {
    /// Converts an [`ErrorKind`] into an [`Error`].
    ///
    /// This conversion creates a new error with a simple representation of error kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// let not_found = ErrorKind::NotFound;
    /// let error = Error::from(not_found);
    /// assert_eq!("entity not found", format!("{error}"));
    /// ```
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error {
            repr: Repr::new_simple(kind),
        }
    }
}

impl Error {
    /// Creates a new I/O error from a known kind of error as well as an
    /// arbitrary error payload.
    ///
    /// This function is used to generically create I/O errors which do not
    /// originate from the OS itself. The `error` argument is an arbitrary
    /// payload which will be contained in this [`Error`].
    ///
    /// Note that this function allocates memory on the heap.
    /// If no extra payload is required, use the `From` conversion from
    /// `ErrorKind`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// // errors can be created from strings
    /// let custom_error = Error::new(ErrorKind::Other, "oh no!");
    ///
    /// // errors can also be created from other errors
    /// let custom_error2 = Error::new(ErrorKind::Interrupted, custom_error);
    ///
    /// // creating an error without payload (and without memory allocation)
    /// let eof_error = Error::from(ErrorKind::UnexpectedEof);
    /// ```
    #[inline(never)]
    #[cfg(feature = "alloc")]
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    /// Creates a new I/O error from an arbitrary error payload.
    ///
    /// This function is used to generically create I/O errors which do not
    /// originate from the OS itself. It is a shortcut for [`Error::new`]
    /// with [`ErrorKind::Other`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    ///
    /// // errors can be created from strings
    /// let custom_error = Error::other("oh no!");
    ///
    /// // errors can also be created from other errors
    /// let custom_error2 = Error::other(custom_error);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn other<E>(error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::_new(ErrorKind::Other, error.into())
    }

    #[cfg(feature = "alloc")]
    fn _new(kind: ErrorKind, error: Box<dyn error::Error + Send + Sync>) -> Error {
        Error {
            repr: Repr::new_custom(Box::new(Custom { kind, error })),
        }
    }

    /// Creates a new I/O error from a known kind of error as well as a constant
    /// message.
    ///
    /// This function does not allocate.
    ///
    /// You should not use this directly, and instead use the `const_error!`
    /// macro: `io::const_error!(ErrorKind::Something, "some_message")`.
    ///
    /// This function should maybe change to `from_static_message<const MSG: &'static
    /// str>(kind: ErrorKind)` in the future, when const generics allow that.
    #[inline]
    #[doc(hidden)]
    pub const fn from_static_message(msg: &'static SimpleMessage) -> Error {
        Self {
            repr: Repr::new_simple_message(msg),
        }
    }

    /// Returns an error representing the last OS error which occurred.
    ///
    /// This function reads the value of `errno` for the target platform (e.g.
    /// `GetLastError` on Windows) and will return a corresponding instance of
    /// [`Error`] for the error code.
    ///
    /// This should be called immediately after a call to a platform function,
    /// otherwise the state of the error value is indeterminate. In particular,
    /// other standard library functions may call platform functions that may
    /// (or may not) reset the error value even if they succeed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    ///
    /// let os_error = Error::last_os_error();
    /// println!("last OS error: {os_error:?}");
    /// ```
    #[doc(alias = "GetLastError")]
    #[doc(alias = "errno")]
    #[must_use]
    #[inline]
    #[cfg(feature = "std")]
    pub fn last_os_error() -> Error {
        Error::from_raw_os_error(sys::io::errno())
    }

    /// Creates a new instance of an [`Error`] from a particular OS error code.
    ///
    /// # Examples
    ///
    /// On Linux:
    ///
    /// ```
    /// # if cfg!(target_os = "linux") {
    /// use std::io;
    ///
    /// let error = io::Error::from_raw_os_error(22);
    /// assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    /// # }
    /// ```
    ///
    /// On Windows:
    ///
    /// ```
    /// # if cfg!(windows) {
    /// use std::io;
    ///
    /// let error = io::Error::from_raw_os_error(10022);
    /// assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    /// # }
    /// ```
    #[must_use]
    #[inline]
    pub fn from_raw_os_error(code: RawOsError) -> Error {
        Error {
            repr: Repr::new_os(code),
        }
    }

    /// Returns the OS error that this error represents (if any).
    ///
    /// If this [`Error`] was constructed via [`last_os_error`] or
    /// [`from_raw_os_error`], then this function will return [`Some`], otherwise
    /// it will return [`None`].
    ///
    /// [`last_os_error`]: Error::last_os_error
    /// [`from_raw_os_error`]: Error::from_raw_os_error
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// fn print_os_error(err: &Error) {
    ///     if let Some(raw_os_err) = err.raw_os_error() {
    ///         println!("raw OS error: {raw_os_err:?}");
    ///     } else {
    ///         println!("Not an OS error");
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Will print "raw OS error: ...".
    ///     print_os_error(&Error::last_os_error());
    ///     // Will print "Not an OS error".
    ///     print_os_error(&Error::new(ErrorKind::Other, "oh no!"));
    /// }
    /// ```
    #[must_use]
    #[inline]
    pub fn raw_os_error(&self) -> Option<RawOsError> {
        match self.repr.data() {
            ErrorData::Os(i) => Some(i),
            ErrorData::Custom(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
        }
    }

    /// Returns a reference to the inner error wrapped by this error (if any).
    ///
    /// If this [`Error`] was constructed via [`new`] then this function will
    /// return [`Some`], otherwise it will return [`None`].
    ///
    /// [`new`]: Error::new
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// fn print_error(err: &Error) {
    ///     if let Some(inner_err) = err.get_ref() {
    ///         println!("Inner error: {inner_err:?}");
    ///     } else {
    ///         println!("No inner error");
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Will print "No inner error".
    ///     print_error(&Error::last_os_error());
    ///     // Will print "Inner error: ...".
    ///     print_error(&Error::new(ErrorKind::Other, "oh no!"));
    /// }
    /// ```
    #[inline]
    #[must_use]
    #[allow(unused_variables)]
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        match self.repr.data() {
            ErrorData::Os(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
            ErrorData::Custom(c) => cfg_select! {
                feature = "alloc" => Some(&*c.error),
                _ => None
            },
        }
    }

    /// Returns a mutable reference to the inner error wrapped by this error
    /// (if any).
    ///
    /// If this [`Error`] was constructed via [`new`] then this function will
    /// return [`Some`], otherwise it will return [`None`].
    ///
    /// [`new`]: Error::new
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{
    ///     error, fmt,
    ///     fmt::Display,
    ///     io::{Error, ErrorKind},
    /// };
    ///
    /// #[derive(Debug)]
    /// struct MyError {
    ///     v: String,
    /// }
    ///
    /// impl MyError {
    ///     fn new() -> MyError {
    ///         MyError {
    ///             v: "oh no!".to_string(),
    ///         }
    ///     }
    ///
    ///     fn change_message(&mut self, new_message: &str) {
    ///         self.v = new_message.to_string();
    ///     }
    /// }
    ///
    /// impl error::Error for MyError {}
    ///
    /// impl Display for MyError {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "MyError: {}", self.v)
    ///     }
    /// }
    ///
    /// fn change_error(mut err: Error) -> Error {
    ///     if let Some(inner_err) = err.get_mut() {
    ///         inner_err
    ///             .downcast_mut::<MyError>()
    ///             .unwrap()
    ///             .change_message("I've been changed!");
    ///     }
    ///     err
    /// }
    ///
    /// fn print_error(err: &Error) {
    ///     if let Some(inner_err) = err.get_ref() {
    ///         println!("Inner error: {inner_err}");
    ///     } else {
    ///         println!("No inner error");
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Will print "No inner error".
    ///     print_error(&change_error(Error::last_os_error()));
    ///     // Will print "Inner error: ...".
    ///     print_error(&change_error(Error::new(ErrorKind::Other, MyError::new())));
    /// }
    /// ```
    #[inline]
    #[must_use]
    #[allow(unused_variables)]
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        match self.repr.data_mut() {
            ErrorData::Os(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
            ErrorData::Custom(c) => cfg_select! {
                feature = "alloc" => Some(&mut *c.error),
                _ => None
            },
        }
    }

    /// Consumes the `Error`, returning its inner error (if any).
    ///
    /// If this [`Error`] was constructed via [`new`] or [`other`],
    /// then this function will return [`Some`],
    /// otherwise it will return [`None`].
    ///
    /// [`new`]: Error::new
    /// [`other`]: Error::other
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// fn print_error(err: Error) {
    ///     if let Some(inner_err) = err.into_inner() {
    ///         println!("Inner error: {inner_err}");
    ///     } else {
    ///         println!("No inner error");
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Will print "No inner error".
    ///     print_error(Error::last_os_error());
    ///     // Will print "Inner error: ...".
    ///     print_error(Error::new(ErrorKind::Other, "oh no!"));
    /// }
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self.repr.into_data() {
            ErrorData::Os(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
            ErrorData::Custom(c) => Some(c.error),
        }
    }

    /// Attempts to downcast the custom boxed error to `E`.
    ///
    /// If this [`Error`] contains a custom boxed error,
    /// then it would attempt downcasting on the boxed error,
    /// otherwise it will return [`Err`].
    ///
    /// If the custom boxed error has the same type as `E`, it will return [`Ok`],
    /// otherwise it will also return [`Err`].
    ///
    /// This method is meant to be a convenience routine for calling
    /// `Box<dyn Error + Sync + Send>::downcast` on the custom boxed error, returned by
    /// [`Error::into_inner`].
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{error::Error, fmt, io};
    ///
    /// #[derive(Debug)]
    /// enum E {
    ///     Io(io::Error),
    ///     SomeOtherVariant,
    /// }
    ///
    /// impl fmt::Display for E {
    ///     // ...
    /// #    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    /// #        todo!()
    /// #    }
    /// }
    /// impl Error for E {}
    ///
    /// impl From<io::Error> for E {
    ///     fn from(err: io::Error) -> E {
    ///         err.downcast::<E>().unwrap_or_else(E::Io)
    ///     }
    /// }
    ///
    /// impl From<E> for io::Error {
    ///     fn from(err: E) -> io::Error {
    ///         match err {
    ///             E::Io(io_error) => io_error,
    ///             e => io::Error::new(io::ErrorKind::Other, e),
    ///         }
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let e = E::SomeOtherVariant;
    /// // Convert it to an io::Error
    /// let io_error = io::Error::from(e);
    /// // Cast it back to the original variant
    /// let e = E::from(io_error);
    /// assert!(matches!(e, E::SomeOtherVariant));
    ///
    /// let io_error = io::Error::from(io::ErrorKind::AlreadyExists);
    /// // Convert it to E
    /// let e = E::from(io_error);
    /// // Cast it back to the original variant
    /// let io_error = io::Error::from(e);
    /// assert_eq!(io_error.kind(), io::ErrorKind::AlreadyExists);
    /// assert!(io_error.get_ref().is_none());
    /// assert!(io_error.raw_os_error().is_none());
    /// # }
    /// ```
    pub fn downcast<E>(self) -> result::Result<E, Self>
    where
        E: error::Error + Send + Sync + 'static,
    {
        #[cfg(feature = "alloc")]
        {
            if let ErrorData::Custom(c) = self.repr.data()
                && c.error.is::<E>()
            {
                if let ErrorData::Custom(b) = self.repr.into_data()
                    && let Ok(err) = b.error.downcast::<E>()
                {
                    Ok(*err)
                } else {
                    // Safety: We have just checked that the condition is true
                    unsafe { core::hint::unreachable_unchecked() }
                }
            } else {
                Err(self)
            }
        }

        #[cfg(not(feature = "alloc"))]
        {
            Err(self)
        }
    }

    /// Returns the corresponding [`ErrorKind`] for this error.
    ///
    /// This may be a value set by Rust code constructing custom `io::Error`s,
    /// or if this `io::Error` was sourced from the operating system,
    /// it will be a value inferred from the system's error encoding.
    /// See [`last_os_error`] for more details.
    ///
    /// [`last_os_error`]: Error::last_os_error
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Error, ErrorKind};
    ///
    /// fn print_error(err: Error) {
    ///     println!("{:?}", err.kind());
    /// }
    ///
    /// fn main() {
    ///     // As no error has (visibly) occurred, this may print anything!
    ///     // It likely prints a placeholder for unidentified (non-)errors.
    ///     print_error(Error::last_os_error());
    ///     // Will print "AddrInUse".
    ///     print_error(Error::new(ErrorKind::AddrInUse, "oh no!"));
    /// }
    /// ```
    #[must_use]
    #[inline]
    #[allow(unused_variables)]
    pub fn kind(&self) -> ErrorKind {
        match self.repr.data() {
            ErrorData::Os(code) => {
                #[cfg(feature = "std")]
                {
                    sys::io::decode_error_kind(code)
                }
                #[cfg(not(feature = "std"))]
                {
                    ErrorKind::Uncategorized
                }
            },
            ErrorData::Custom(c) => c.kind,
            ErrorData::Simple(kind) => kind,
            ErrorData::SimpleMessage(m) => m.kind,
        }
    }

    #[inline]
    #[allow(unused_variables)]
    pub(crate) fn is_interrupted(&self) -> bool {
        match self.repr.data() {
            ErrorData::Os(code) => {
                #[cfg(feature = "std")]
                {
                    sys::io::is_interrupted(code)
                }
                #[cfg(not(feature = "std"))]
                {
                    false
                }
            },
            ErrorData::Custom(c) => c.kind == ErrorKind::Interrupted,
            ErrorData::Simple(kind) => kind == ErrorKind::Interrupted,
            ErrorData::SimpleMessage(m) => m.kind == ErrorKind::Interrupted,
        }
    }
}

impl fmt::Debug for Repr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.data() {
            ErrorData::Os(code) => {
                let mut f = fmt.debug_struct("Os");

                f.field("code", &code);

                #[cfg(feature = "std")]
                {
                    f.field("kind", &sys::io::decode_error_kind(code))
                        .field("message", &sys::io::error_string(code));
                }

                f.finish()
            },
            ErrorData::Custom(c) => fmt::Debug::fmt(&c, fmt),
            ErrorData::Simple(kind) => fmt.debug_tuple("Kind").field(&kind).finish(),
            ErrorData::SimpleMessage(msg) => fmt
                .debug_struct("Error")
                .field("kind", &msg.kind)
                .field("message", &msg.message)
                .finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.repr.data() {
            ErrorData::Os(code) => {
                cfg_select! {
                    feature = "std" => {
                        let detail = sys::io::error_string(code);
                        write!(fmt, "{detail} (os error {code})")
                    }
                    _ => {
                        write!(fmt, "os error {code}")
                    }
                }
            },
            ErrorData::Custom(c) => {
                cfg_select! {
                    feature = "alloc" => c.error.fmt(fmt),
                    _ => write!(fmt, "{}", c.kind.as_str())
                }
            },
            ErrorData::Simple(kind) => write!(fmt, "{}", kind.as_str()),
            ErrorData::SimpleMessage(msg) => msg.message.fmt(fmt),
        }
    }
}

impl error::Error for Error {
    #[allow(deprecated)]
    #[allow(unused_variables)]
    fn cause(&self) -> Option<&dyn error::Error> {
        match self.repr.data() {
            ErrorData::Os(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
            ErrorData::Custom(c) => cfg_select! {
                feature = "alloc" => c.error.cause(),
                _ => None
            },
        }
    }

    #[allow(unused_variables)]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.repr.data() {
            ErrorData::Os(..) => None,
            ErrorData::Simple(..) => None,
            ErrorData::SimpleMessage(..) => None,
            ErrorData::Custom(c) => cfg_select! {
                feature = "alloc" => c.error.source(),
                _ => None
            },
        }
    }
}

fn _assert_error_is_sync_send() {
    fn _is_sync_send<T: Sync + Send>() {}
    _is_sync_send::<Error>();
}

#[cfg(test)]
mod tests;
