use super::{Custom, Error, ErrorData, ErrorKind, Repr, SimpleMessage};
use crate::io_const_error;

use core::{assert_matches, error, fmt};

use crate::sys::io::{decode_error_kind, error_string};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, string::ToString};

#[test]
fn test_size() {
    assert!(size_of::<Error>() <= size_of::<[usize; 2]>());
}

#[test]
#[cfg(feature = "std")]
fn test_debug_error() {
    let code = 6;
    let msg = error_string(code);
    let kind = decode_error_kind(code);
    let err = Error::new(ErrorKind::InvalidInput, Error::from_raw_os_error(code));
    let expected = alloc::format!(
        "Custom {{ kind: InvalidInput, error: Os {{ code: {:?}, kind: {:?}, message: {:?} }} }}",
        code,
        kind,
        msg
    );
    assert_eq!(alloc::format!("{err:?}"), expected);
}

#[test]
#[cfg(feature = "alloc")]
fn test_downcasting() {
    #[derive(Debug)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("asdf")
        }
    }

    impl error::Error for TestError {}

    // we have to call all of these UFCS style right now since method
    // resolution won't implicitly drop the Send+Sync bounds
    let mut err = Error::new(ErrorKind::Other, TestError);
    assert!(err.get_ref().unwrap().is::<TestError>());
    assert_eq!("asdf", err.get_ref().unwrap().to_string());
    assert!(err.get_mut().unwrap().is::<TestError>());
    let extracted = err.into_inner().unwrap();
    extracted.downcast::<TestError>().unwrap();
}

#[test]
#[cfg(feature = "alloc")]
fn test_const() {
    const E: Error = io_const_error!(ErrorKind::NotFound, "hello");

    assert_eq!(E.kind(), ErrorKind::NotFound);
    assert_eq!(E.to_string(), "hello");
    assert!(alloc::format!("{E:?}").contains("\"hello\""));
    assert!(alloc::format!("{E:?}").contains("NotFound"));
}

#[test]
fn test_os_packing() {
    for code in -20..20 {
        let e = Error::from_raw_os_error(code);
        assert_eq!(e.raw_os_error(), Some(code));
        assert_matches!(
            e.repr.data(),
            ErrorData::Os(c) if c == code,
        );
    }
}

#[test]
fn test_errorkind_packing() {
    assert_eq!(Error::from(ErrorKind::NotFound).kind(), ErrorKind::NotFound);
    assert_eq!(
        Error::from(ErrorKind::PermissionDenied).kind(),
        ErrorKind::PermissionDenied
    );
    assert_eq!(Error::from(ErrorKind::Uncategorized).kind(), ErrorKind::Uncategorized);
    // Check that the innards look like what we want.
    assert_matches!(
        Error::from(ErrorKind::OutOfMemory).repr.data(),
        ErrorData::Simple(ErrorKind::OutOfMemory),
    );
}

#[test]
#[cfg(feature = "alloc")]
fn test_simple_message_packing() {
    use super::{ErrorKind::*, SimpleMessage};
    macro_rules! check_simple_msg {
        ($err:expr, $kind:ident, $msg:literal) => {{
            let e = &$err;
            // Check that the public api is right.
            assert_eq!(e.kind(), $kind);
            assert!(alloc::format!("{e:?}").contains($msg));
            // and we got what we expected
            assert_matches!(
                e.repr.data(),
                ErrorData::SimpleMessage(SimpleMessage {
                    kind: $kind,
                    message: $msg
                })
            );
        }};
    }

    let not_static = io_const_error!(Uncategorized, "not a constant!");
    check_simple_msg!(not_static, Uncategorized, "not a constant!");

    const CONST: Error = io_const_error!(NotFound, "definitely a constant!");
    check_simple_msg!(CONST, NotFound, "definitely a constant!");

    static STATIC: Error = io_const_error!(BrokenPipe, "a constant, sort of!");
    check_simple_msg!(STATIC, BrokenPipe, "a constant, sort of!");
}

#[derive(Debug, PartialEq)]
struct Bojji(bool);
impl error::Error for Bojji {}
impl fmt::Display for Bojji {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ah! {:?}", self)
    }
}

#[test]
#[cfg(feature = "alloc")]
fn test_custom_error_packing() {
    use super::Custom;
    let test = Error::new(ErrorKind::Uncategorized, Bojji(true));
    assert_eq!(test.kind(), ErrorKind::Uncategorized);
    assert_matches!(
        test.get_ref(),
        Some(error) if error.downcast_ref::<Bojji>() == Some(&Bojji(true)),
    );
}

#[derive(Debug)]
struct E;

impl fmt::Display for E {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl error::Error for E {}

#[test]
#[cfg(feature = "alloc")]
fn test_std_io_error_downcast() {
    // Case 1: custom error, downcast succeeds
    let io_error = Error::new(ErrorKind::Other, Bojji(true));
    let e: Bojji = io_error.downcast().unwrap();
    assert!(e.0);

    // Case 2: custom error, downcast fails
    let io_error = Error::new(ErrorKind::Other, Bojji(true));
    let io_error = io_error.downcast::<E>().unwrap_err();

    //   ensures that the custom error is intact
    assert_eq!(ErrorKind::Other, io_error.kind());
    let e: Bojji = io_error.downcast().unwrap();
    assert!(e.0);

    // Case 3: os error
    let errno = 20;
    let io_error = Error::from_raw_os_error(errno);
    let io_error = io_error.downcast::<E>().unwrap_err();

    assert_eq!(errno, io_error.raw_os_error().unwrap());

    // Case 4: simple
    let kind = ErrorKind::OutOfMemory;
    let io_error: Error = kind.into();
    let io_error = io_error.downcast::<E>().unwrap_err();

    assert_eq!(kind, io_error.kind());

    // Case 5: simple message
    const SIMPLE_MESSAGE: SimpleMessage = SimpleMessage {
        kind: ErrorKind::Other,
        message: "simple message error test",
    };
    let io_error = Error::from_static_message(&SIMPLE_MESSAGE);
    let io_error = io_error.downcast::<E>().unwrap_err();

    assert_eq!(SIMPLE_MESSAGE.kind, io_error.kind());
    #[cfg(feature = "alloc")]
    {
        assert_eq!(SIMPLE_MESSAGE.message, alloc::format!("{io_error}"));
    }
}
