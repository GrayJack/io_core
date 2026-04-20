#![forbid(unsafe_op_in_unsafe_fn)]

cfg_select! {
    all(unix, feature = "std") => {
        mod unix;
        pub use unix::{Pipe, pipe};
    }
    all(windows, feature = "std") => {
        mod windows;
        pub use windows::{Pipe, pipe};
    }
    all(target_os = "motor", feature = "std") => {
        mod motor;
        pub use motor::{Pipe, pipe};
    }
    _ => {
        mod unsupported;
        pub use unsupported::{Pipe, pipe};
    }
}
