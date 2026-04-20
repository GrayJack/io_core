- Buf reader/writer backed by array (and const generics)
- Associated error in traits
- Maintain io::Error Os kind variant, but make the required os-dependent functionality as a new trait.


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


```rust
pub trait OsError {
    fn to_raw(&self) -> u32/usize;

    /* ... */
}
```
