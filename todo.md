- Associated error in traits (?)


### Associated error in traits ideas

- Introduce `Io` trait that defines the associated Error for the type. Roughly:

```rust
pub trait Io {
    /// Maybe defaulting to
    type Error: error::Error + Into<io::Error> + TryFrom<io::Error> + TryFrom<io::Error, Error=>;
}

pub trait Read: Io { /*...*/ }
pub trait Write: Io { /*...*/ }
```
