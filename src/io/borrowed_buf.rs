use core::{
    cmp,
    fmt::{self, Debug, Formatter},
    mem::{self, MaybeUninit},
    ptr,
};

#[cfg(test)]
mod tests;

/// A borrowed byte buffer which is incrementally filled.
///
/// This type makes it safer to work with `MaybeUninit` buffers, such as to read into a buffer
/// without having to initialize it first. It tracks the region of bytes that have been filled and
/// whether the unfilled region was initialized.
///
/// In summary, the contents of the buffer can be visualized as:
/// ```not_rust
/// [                capacity                ]
/// [ filled | unfilled (may be initialized) ]
/// ```
///
/// A `BorrowedBuf` is created around some existing data (or capacity for data) via a unique
/// reference (`&mut`). The `BorrowedBuf` can be configured (e.g., using `clear` or `set_init`), but
/// cannot be directly written. To write into the buffer, use `unfilled` to create a
/// `BorrowedCursor`. The cursor has write-only access to the unfilled portion of the buffer (you
/// can think of it as a write-only iterator).
///
/// The lifetime `'data` is a bound on the lifetime of the underlying data.
pub struct BorrowedBuf<'data> {
    /// The buffer's underlying data.
    buf: &'data mut [MaybeUninit<u8>],
    /// The length of `self.buf` which is known to be filled.
    filled: usize,
    /// Whether the entire unfilled part of `self.buf` has explicitly been initialized.
    init: bool,
}

impl Debug for BorrowedBuf<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedBuf")
            .field("init", &self.init)
            .field("filled", &self.filled)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// Creates a new `BorrowedBuf` from a fully initialized slice.
impl<'data> From<&'data mut [u8]> for BorrowedBuf<'data> {
    #[inline]
    fn from(slice: &'data mut [u8]) -> BorrowedBuf<'data> {
        BorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(slice as *mut [u8] as *mut [MaybeUninit<u8>]) },
            filled: 0,
            init: true,
        }
    }
}

/// Creates a new `BorrowedBuf` from an uninitialized buffer.
impl<'data> From<&'data mut [MaybeUninit<u8>]> for BorrowedBuf<'data> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<u8>]) -> BorrowedBuf<'data> {
        BorrowedBuf {
            buf,
            filled: 0,
            init: false,
        }
    }
}

/// Creates a new `BorrowedBuf` from a fully initialized array.
impl<'data, const N: usize> From<&'data mut [u8; N]> for BorrowedBuf<'data> {
    #[inline]
    fn from(array: &'data mut [u8; N]) -> BorrowedBuf<'data> {
        BorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(array as *mut [u8] as *mut [MaybeUninit<u8>]) },
            filled: 0,
            init: true,
        }
    }
}

/// Creates a new `BorrowedBuf` from an uninitialized buffer array.
impl<'data, const N: usize> From<&'data mut [MaybeUninit<u8>; N]> for BorrowedBuf<'data> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<u8>; N]) -> BorrowedBuf<'data> {
        BorrowedBuf {
            buf,
            filled: 0,
            init: false,
        }
    }
}

/// Creates a new `BorrowedBuf` from a cursor.
///
/// Use `BorrowedCursor::with_unfilled_buf` instead for a safer alternative.
impl<'data> From<BorrowedCursor<'data>> for BorrowedBuf<'data> {
    #[inline]
    fn from(buf: BorrowedCursor<'data>) -> BorrowedBuf<'data> {
        BorrowedBuf {
            // SAFETY: no initialized byte is ever uninitialized as per
            // `BorrowedBuf`'s invariant
            buf: unsafe { buf.buf.buf.get_unchecked_mut(buf.buf.filled..) },
            filled: 0,
            init: buf.buf.init,
        }
    }
}

impl<'data> BorrowedBuf<'data> {
    /// Returns the total capacity of the buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Returns the length of the filled part of the buffer.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.filled
    }

    /// Returns the length of the initialized part of the buffer.
    #[inline]
    pub fn is_init(&self) -> bool {
        self.init
    }

    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a shared reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled(self) -> &'data [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled_mut(self) -> &'data mut [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a cursor over the unfilled part of the buffer.
    #[inline]
    pub fn unfilled<'this>(&'this mut self) -> BorrowedCursor<'this> {
        BorrowedCursor {
            // SAFETY: we never assign into `BorrowedCursor::buf`, so treating its
            // lifetime covariantly is safe.
            buf: unsafe {
                mem::transmute::<&'this mut BorrowedBuf<'data>, &'this mut BorrowedBuf<'this>>(self)
            },
        }
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The contents of the buffer are not modified.
    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.filled = 0;
        self
    }

    /// Asserts that the unfilled part of the buffer is initialized.
    ///
    /// # Safety
    ///
    /// All the bytes of the buffer must be initialized.
    #[inline]
    pub unsafe fn set_init(&mut self) -> &mut Self {
        self.init = true;
        self
    }
}

/// A writeable view of the unfilled portion of a [`BorrowedBuf`].
///
/// The unfilled portion may be uninitialized; see [`BorrowedBuf`] for details.
///
/// Data can be written directly to the cursor by using [`append`](BorrowedCursor::append) or
/// indirectly by getting a slice of part or all of the cursor and writing into the slice. In the
/// indirect case, the caller must call [`advance`](BorrowedCursor::advance) after writing to inform
/// the cursor how many bytes have been written.
///
/// Once data is written to the cursor, it becomes part of the filled portion of the underlying
/// `BorrowedBuf` and can no longer be accessed or re-written by the cursor. I.e., the cursor tracks
/// the unfilled part of the underlying `BorrowedBuf`.
///
/// The lifetime `'a` is a bound on the lifetime of the underlying buffer (which means it is a bound
/// on the data in that buffer by transitivity).
#[derive(Debug)]
pub struct BorrowedCursor<'a> {
    /// The underlying buffer.
    // Safety invariant: we treat the type of buf as covariant in the lifetime of `BorrowedBuf`
    // when we create a `BorrowedCursor`. This is only safe if we never replace `buf` by
    // assigning into it, so don't do that!
    buf: &'a mut BorrowedBuf<'a>,
}

impl<'a> BorrowedCursor<'a> {
    /// Reborrows this cursor by cloning it with a smaller lifetime.
    ///
    /// Since a cursor maintains unique access to its underlying buffer, the borrowed cursor is
    /// not accessible while the new cursor exists.
    #[inline]
    pub fn reborrow<'this>(&'this mut self) -> BorrowedCursor<'this> {
        BorrowedCursor {
            // SAFETY: we never assign into `BorrowedCursor::buf`, so treating its
            // lifetime covariantly is safe.
            buf: unsafe {
                mem::transmute::<&'this mut BorrowedBuf<'a>, &'this mut BorrowedBuf<'this>>(
                    self.buf,
                )
            },
        }
    }

    /// Returns the available space in the cursor.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.capacity() - self.buf.filled
    }

    /// Returns the number of bytes written to the `BorrowedBuf` this cursor was created from.
    ///
    /// In particular, the count returned is shared by all reborrows of the cursor.
    #[inline]
    pub fn written(&self) -> usize {
        self.buf.filled
    }

    /// Returns `true` if the buffer is initialized.
    #[inline]
    pub fn is_init(&self) -> bool {
        self.buf.init
    }

    /// Set the buffer as fully initialized.
    ///
    /// # Safety
    ///
    /// All the bytes of the cursor must be initialized.
    #[inline]
    pub unsafe fn set_init(&mut self) {
        self.buf.init = true;
    }

    /// Returns a mutable reference to the whole cursor.
    ///
    /// # Safety
    ///
    /// The caller must not uninitialize any bytes of the cursor if it is initialized.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: always in bounds
        unsafe { self.buf.buf.get_unchecked_mut(self.buf.filled..) }
    }

    /// Advances the cursor by asserting that `n` bytes have been filled.
    ///
    /// After advancing, the `n` bytes are no longer accessible via the cursor and can only be
    /// accessed via the underlying buffer. I.e., the buffer's filled portion grows by `n` elements
    /// and its unfilled portion (and the capacity of this cursor) shrinks by `n` elements.
    ///
    /// If less than `n` bytes initialized (by the cursor's point of view), `set_init` should be
    /// called first.
    ///
    /// # Panics
    ///
    /// Panics if there are less than `n` bytes initialized.
    #[inline]
    pub fn advance_checked(&mut self, n: usize) -> &mut Self {
        // The subtraction cannot underflow by invariant of this type.
        let init_unfilled = if self.buf.init {
            self.buf.buf.len() - self.buf.filled
        } else {
            0
        };
        assert!(n <= init_unfilled);

        self.buf.filled += n;
        self
    }

    /// Advances the cursor by asserting that `n` bytes have been filled.
    ///
    /// After advancing, the `n` bytes are no longer accessible via the cursor and can only be
    /// accessed via the underlying buffer. I.e., the buffer's filled portion grows by `n` elements
    /// and its unfilled portion (and the capacity of this cursor) shrinks by `n` elements.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` bytes of the cursor have been properly
    /// initialised.
    #[inline]
    pub unsafe fn advance(&mut self, n: usize) -> &mut Self {
        self.buf.filled += n;
        self
    }

    /// Initializes all bytes in the cursor and returns them.
    #[inline]
    pub fn ensure_init(&mut self) -> &mut [u8] {
        // SAFETY: always in bounds and we never uninitialize these bytes.
        let unfilled = unsafe { self.buf.buf.get_unchecked_mut(self.buf.filled..) };

        if !self.buf.init {
            // SAFETY: 0 is a valid value for MaybeUninit<u8> and the length matches the allocation
            // since it is comes from a slice reference.
            unsafe {
                ptr::write_bytes(unfilled.as_mut_ptr(), 0, unfilled.len());
            }
            self.buf.init = true;
        }

        // SAFETY: these bytes have just been initialized if they weren't before
        unsafe { unfilled.assume_init_mut() }
    }

    /// Appends data to the cursor, advancing position within its buffer.
    ///
    /// # Panics
    ///
    /// Panics if `self.capacity()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[u8]) {
        assert!(self.capacity() >= buf.len());

        // SAFETY: we do not de-initialize any of the elements of the slice
        unsafe {
            self.as_mut()[..buf.len()].write_copy_of_slice(buf);
        }

        self.buf.filled += buf.len();
    }

    /// Runs the given closure with a `BorrowedBuf` containing the unfilled part
    /// of the cursor.
    ///
    /// This enables inspecting what was written to the cursor.
    ///
    /// # Panics
    ///
    /// Panics if the `BorrowedBuf` given to the closure is replaced by another
    /// one.
    pub fn with_unfilled_buf<T>(&mut self, f: impl FnOnce(&mut BorrowedBuf<'_>) -> T) -> T {
        let mut buf = BorrowedBuf::from(self.reborrow());
        let prev_ptr = buf.buf as *const _;
        let res = f(&mut buf);

        // Check that the caller didn't replace the `BorrowedBuf`.
        // This is necessary for the safety of the code below: if the check wasn't
        // there, one could mark some bytes as initialized even though there aren't.
        assert!(core::ptr::eq(prev_ptr, buf.buf));

        let filled = buf.filled;
        let init = buf.init;

        // Update `init` and `filled` fields with what was written to the buffer.
        // `self.buf.filled` was the starting length of the `BorrowedBuf`.
        //
        // SAFETY: These amounts of bytes were initialized/filled in the `BorrowedBuf`,
        // and therefore they are initialized/filled in the cursor too, because the
        // buffer wasn't replaced.
        self.buf.init = init;
        self.buf.filled += filled;

        res
    }
}

/// A borrowed byte buffer which is incrementally filled and initialized.
///
/// This type is a sort of "double cursor". It tracks three regions in the buffer: a region at the
/// beginning of the buffer that has been logically filled with data, a region that has been
/// initialized at some point but not yet logically filled, and a region at the end that is fully
/// uninitialized. The filled region is guaranteed to be a subset of the initialized region.
///
/// In summary, the contents of the buffer can be visualized as:
/// ```not_rust
/// [             capacity              ]
/// [ filled |         unfilled         ]
/// [    initialized    | uninitialized ]
/// ```
///
/// A `TrackingBorrowedBuf` is created around some existing data (or capacity for data) via a unique
/// reference (`&mut`). The `TrackingBorrowedBuf` can be configured (e.g., using `clear` or
/// `set_init`), but cannot be directly written. To write into the buffer, use `unfilled` to create
/// a `TrackingBorrowedCursor`. The cursor has write-only access to the unfilled portion of the
/// buffer (you can think of it as a write-only iterator).
///
/// The lifetime `'data` is a bound on the lifetime of the underlying data.
pub struct TrackingBorrowedBuf<'data> {
    /// The buffer's underlying data.
    buf: &'data mut [MaybeUninit<u8>],
    /// The length of `self.buf` which is known to be filled.
    filled: usize,
    /// The length of `self.buf` which is known to be initialized.
    init: usize,
}

impl fmt::Debug for TrackingBorrowedBuf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedBufTracking")
            .field("init", &self.init)
            .field("filled", &self.filled)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// Create a new `TrackingBorrowedBuf` from a fully initialized slice.
impl<'data> From<&'data mut [u8]> for TrackingBorrowedBuf<'data> {
    #[inline]
    fn from(slice: &'data mut [u8]) -> TrackingBorrowedBuf<'data> {
        let len = slice.len();

        TrackingBorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            // buf: unsafe { (slice as *mut [u8]).as_uninit_slice_mut().unwrap() },
            buf: unsafe {
                core::slice::from_raw_parts_mut((slice as *mut [u8]) as *mut MaybeUninit<u8>, len)
            },
            filled: 0,
            init: len,
        }
    }
}

/// Create a new `TrackingBorrowedBuf` from an uninitialized buffer.
///
/// Use `set_init` if part of the buffer is known to be already initialized.
impl<'data> From<&'data mut [MaybeUninit<u8>]> for TrackingBorrowedBuf<'data> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<u8>]) -> TrackingBorrowedBuf<'data> {
        TrackingBorrowedBuf {
            buf,
            filled: 0,
            init: 0,
        }
    }
}

/// Creates a new `TrackingBorrowedBuf` from a fully initialized array.
impl<'data, const N: usize> From<&'data mut [u8; N]> for TrackingBorrowedBuf<'data> {
    #[inline]
    fn from(array: &'data mut [u8; N]) -> TrackingBorrowedBuf<'data> {
        TrackingBorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(array as *mut [u8] as *mut [MaybeUninit<u8>]) },
            filled: 0,
            init: N,
        }
    }
}

/// Creates a new `TrackingBorrowedBuf` from an uninitialized buffer array.
impl<'data, const N: usize> From<&'data mut [MaybeUninit<u8>; N]> for TrackingBorrowedBuf<'data> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<u8>; N]) -> TrackingBorrowedBuf<'data> {
        TrackingBorrowedBuf {
            buf,
            filled: 0,
            init: 0,
        }
    }
}

/// Creates a new `TrackingBorrowedBuf` from a cursor.
///
/// Use `TrackingBorrowedCursor::with_unfilled_buf` instead for a safer alternative.
impl<'data> From<TrackingBorrowedCursor<'data>> for TrackingBorrowedBuf<'data> {
    #[inline]
    fn from(buf: TrackingBorrowedCursor<'data>) -> TrackingBorrowedBuf<'data> {
        TrackingBorrowedBuf {
            // SAFETY: no initialized byte is ever uninitialized as per
            // `BorrowedBuf`'s invariant
            buf: unsafe { buf.buf.buf.get_unchecked_mut(buf.buf.filled..) },
            filled: 0,
            init: buf.buf.init,
        }
    }
}

impl<'data> TrackingBorrowedBuf<'data> {
    /// Returns the total capacity of the buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Returns the length of the filled part of the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.filled
    }

    /// Returns `true` if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Returns the length of the initialized part of the buffer.
    #[inline]
    pub fn init_len(&self) -> usize {
        self.init
    }

    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a shared reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled(self) -> &'data [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled_mut(self) -> &'data mut [u8] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a cursor over the unfilled part of the buffer.
    #[inline]
    pub fn unfilled<'this>(&'this mut self) -> TrackingBorrowedCursor<'this> {
        TrackingBorrowedCursor {
            start: self.filled,
            // SAFETY: we never assign into `BorrowedCursor::buf`, so treating its
            // lifetime covariantly is safe.
            buf: unsafe {
                mem::transmute::<
                    &'this mut TrackingBorrowedBuf<'data>,
                    &'this mut TrackingBorrowedBuf<'this>,
                >(self)
            },
        }
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The number of initialized bytes is not changed, and the contents of the buffer are not
    /// modified.
    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.filled = 0;
        self
    }

    /// Asserts that the first `n` bytes of the buffer are initialized.
    ///
    /// `BorrowedBuf` assumes that bytes are never de-initialized, so this method does nothing when
    /// called with fewer bytes than are already known to be initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` unfilled bytes of the buffer have already been
    /// initialized.
    #[inline]
    pub unsafe fn set_init(&mut self, n: usize) -> &mut Self {
        self.init = cmp::max(self.init, n);
        self
    }
}

/// A writeable view of the unfilled portion of a [`TrackingBorrowedBuf`].
///
/// Provides access to the initialized and uninitialized parts of the underlying
/// `TrackingBorrowedBuf`. Data can be written directly to the cursor by using
/// [`append`](TrackingBorrowedCursor::append) or indirectly by getting a slice of part or all of
/// the cursor and writing into the slice. In the indirect case, the caller must call
/// [`advance`](TrackingBorrowedCursor::advance) after writing to inform the cursor how many bytes
/// have been written.
///
/// Once data is written to the cursor, it becomes part of the filled portion of the underlying
/// `TrackingBorrowedBuf` and can no longer be accessed or re-written by the cursor. I.e., the
/// cursor tracks the unfilled part of the underlying `TrackingBorrowedBuf`.
///
/// The lifetime `'a` is a bound on the lifetime of the underlying buffer (which means it is a bound
/// on the data in that buffer by transitivity).
#[derive(Debug)]
pub struct TrackingBorrowedCursor<'a> {
    /// The underlying buffer.
    // Safety invariant: we treat the type of buf as covariant in the lifetime of `BorrowedBuf`
    // when we create a `BorrowedCursor`. This is only safe if we never replace `buf` by
    // assigning into it, so don't do that!
    buf: &'a mut TrackingBorrowedBuf<'a>,
    /// The length of the filled portion of the underlying buffer at the time of the cursor's
    /// creation.
    start: usize,
}

impl<'a> TrackingBorrowedCursor<'a> {
    /// Reborrows this cursor by cloning it with a smaller lifetime.
    ///
    /// Since a cursor maintains unique access to its underlying buffer, the borrowed cursor is
    /// not accessible while the new cursor exists.
    #[inline]
    pub fn reborrow<'this>(&'this mut self) -> TrackingBorrowedCursor<'this> {
        TrackingBorrowedCursor {
            // SAFETY: we never assign into `BorrowedCursor::buf`, so treating its
            // lifetime covariantly is safe.
            buf: unsafe {
                mem::transmute::<
                    &'this mut TrackingBorrowedBuf<'a>,
                    &'this mut TrackingBorrowedBuf<'this>,
                >(self.buf)
            },
            start: self.start,
        }
    }

    /// Returns the available space in the cursor.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.capacity() - self.buf.filled
    }

    /// Returns the number of bytes written to this cursor since it was created from a
    /// `BorrowedBuf`.
    ///
    /// Note that if this cursor is a reborrowed clone of another, then the count returned is the
    /// count written via either cursor, not the count since the cursor was reborrowed.
    #[inline]
    pub fn written(&self) -> usize {
        self.buf.filled - self.start
    }

    /// Returns a shared reference to the initialized portion of the cursor.
    #[inline]
    pub fn init_ref(&self) -> &[u8] {
        // SAFETY: We only slice the initialized part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.buf.get_unchecked(self.buf.filled..self.buf.init);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the initialized portion of the cursor.
    #[inline]
    pub fn init_mut(&mut self) -> &mut [u8] {
        // SAFETY: We only slice the initialized part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.buf.get_unchecked_mut(self.buf.filled..self.buf.init);
            buf.assume_init_mut()
        }
    }

    /// Returns a mutable reference to the uninitialized part of the cursor.
    ///
    /// It is safe to uninitialize any of these bytes.
    #[inline]
    pub fn uninit_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: always in bounds
        unsafe { self.buf.buf.get_unchecked_mut(self.buf.init..) }
    }

    /// Returns a mutable reference to the whole cursor.
    ///
    /// # Safety
    ///
    /// The caller must not uninitialize any bytes in the initialized portion of the cursor.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: always in bounds
        unsafe { self.buf.buf.get_unchecked_mut(self.buf.filled..) }
    }

    /// Advance the cursor by asserting that `n` bytes have been filled.
    ///
    /// After advancing, the `n` bytes are no longer accessible via the cursor and can only be
    /// accessed via the underlying buffer. I.e., the buffer's filled portion grows by `n` elements
    /// and its unfilled portion (and the capacity of this cursor) shrinks by `n` elements.
    ///
    /// If less than `n` bytes initialized (by the cursor's point of view), `set_init` should be
    /// called first.
    ///
    /// # Panics
    ///
    /// Panics if there are less than `n` bytes initialized.
    #[inline]
    pub fn advance(&mut self, n: usize) -> &mut Self {
        let filled = self.buf.filled.strict_add(n);
        assert!(filled <= self.buf.init);

        self.buf.filled = filled;
        self
    }

    /// Advance the cursor by asserting that `n` bytes have been filled.
    ///
    /// After advancing, the `n` bytes are no longer accessible via the cursor and can only be
    /// accessed via the underlying buffer. I.e., the buffer's filled portion grows by `n` elements
    /// and its unfilled portion (and the capacity of this cursor) shrinks by `n` elements.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` bytes of the cursor have been properly
    /// initialized.
    #[inline]
    pub unsafe fn advance_unchecked(&mut self, n: usize) -> &mut Self {
        self.buf.filled += n;
        self.buf.init = cmp::max(self.buf.init, self.buf.filled);
        self
    }

    /// Initializes all bytes in the cursor.
    #[inline]
    pub fn ensure_init(&mut self) -> &mut Self {
        let uninit = self.uninit_mut();
        // SAFETY: 0 is a valid value for MaybeUninit<u8> and the length matches the allocation
        // since it is comes from a slice reference.
        unsafe {
            ptr::write_bytes(uninit.as_mut_ptr(), 0, uninit.len());
        }
        self.buf.init = self.buf.capacity();

        self
    }

    /// Asserts that the first `n` unfilled bytes of the cursor are initialized.
    ///
    /// `BorrowedBuf` assumes that bytes are never de-initialized, so this method does nothing when
    /// called with fewer bytes than are already known to be initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` bytes of the buffer have already been initialized.
    #[inline]
    pub unsafe fn set_init(&mut self, n: usize) -> &mut Self {
        self.buf.init = cmp::max(self.buf.init, self.buf.filled + n);
        self
    }

    /// Appends data to the cursor, advancing position within its buffer.
    ///
    /// # Panics
    ///
    /// Panics if `self.capacity()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[u8]) {
        assert!(self.capacity() >= buf.len());

        // SAFETY: we do not de-initialize any of the elements of the slice
        unsafe {
            self.as_mut()[..buf.len()].write_copy_of_slice(buf);
        }

        // SAFETY: We just added the entire contents of buf to the filled section.
        unsafe {
            self.set_init(buf.len());
        }
        self.buf.filled += buf.len();
    }
}
