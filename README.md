# I/O Core

This is a `no_std` I/O foundation for Rust. It provides standard-library-style I/O traits and adapters in `no_std` contexts (that came from Rust std for the most part).

This aims to import/add as much as possible from `std::io`, adapting to non-alloc context as possible.

## Features

- `alloc` - enables heap-backed types and buffered adapters
- `nightly` - enables APIs that depends of nightly features and nightly-only optimizations
<!-- - ~~`std` - enables `alloc` and integrates with the standard library (this is not workingas of now)~~ -->