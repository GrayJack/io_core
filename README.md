# I/O Core

This is a `no_std` I/O foundation for Rust. It provides standard-library-style I/O traits and adapters in `no_std` contexts (that came from Rust std for the most part).

This aims to import/add as much as possible from `std::io`, adapting to non-alloc context as possible, but still optionally allowing to enable alloc usage.

## Features

- `alloc` - enables heap-backed types and buffered adapters
- `nightly` - enables APIs that depends of nightly features and nightly-only optimizations
<!-- - ~~`std` - enables `alloc` and integrates with the standard library (this is not workingas of now)~~ -->

## Additions

- Includes `ArrayBufReader`, `ArrayBufWriter`, `ArrayLineWriter` as array backed buffered reader and writer, available in `no_std`.
- Includes `TrackingBorrowedBuf`, `TrackingBorrowedCursor` as variants of `BorrowedBuf` and `BorrowedCursor` that tracks initialized data **and** filled data.
  - This was the older behavior of `BorrowedBuf` and `BorrowedCursor`, but for performance reasons inside the rust core/std was modified to only track filled data.
  - This was reintroduced as they can be useful for some more specific cases not used on the Rust core/std.