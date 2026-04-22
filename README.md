# I/O Core

This is a `no_std` I/O foundation for Rust. It provides standard-library-style I/O traits and adapters in `no_std` contexts (that came from Rust std for the most part).

This aims to import/add as much as possible from `std::io`, adapting to non-alloc context as possible, but still optionally allowing to enable alloc usage.

## Features

- `alloc` - Enables heap-backed types and buffered adapters
- `nightly` - Enables APIs that depends of nightly features and nightly-only optimizations
- `std` - Enables same as `alloc` and integrates this crate types with std traits and vice-versa

## Additions

- Includes `ArrayBufReader`, `ArrayBufWriter`, `ArrayLineWriter` as array backed buffered reader and writer, available in `no_std`.
- Includes `TrackingBorrowedBuf`, `TrackingBorrowedCursor` as variants of `BorrowedBuf` and `BorrowedCursor` that tracks initialized data **and** filled data.
  - This was the older behavior of `BorrowedBuf` and `BorrowedCursor`, but for performance reasons inside the rust core/std was modified to only track filled data.
  - This was reintroduced as they can be useful for some more specific cases not used on the Rust core/std.
- Includes `OsFunctions` type and `set_os_functions` function as a form of user-configurable handling of OS integration to be used by the crate (for now it is only for `Error`, potentially other features in the future).
  - This can be used to implement some OS support for OS not supported by this crate or for a custom/niche OS.

## License

This project is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).