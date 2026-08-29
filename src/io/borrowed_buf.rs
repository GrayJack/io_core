use core::{
    cmp,
    fmt::{self, Debug, Formatter},
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
    slice,
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
pub struct BorrowedBuf<'data, T> {
    /// The buffer's underlying data.
    buf: &'data mut [MaybeUninit<T>],
    /// The length of `self.buf` which is known to be filled.
    filled: usize,
    /// Whether the entire unfilled part of `self.buf` has explicitly been initialized.
    init: bool,
}

impl<T> Debug for BorrowedBuf<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        BorrowedBufDebug {
            init: self.init,
            filled: self.filled,
            capacity: self.capacity(),
        }
        .fmt(f)
    }
}

struct BorrowedBufDebug {
    init: bool,
    filled: usize,
    capacity: usize,
}

impl Debug for BorrowedBufDebug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedBuf")
            .field("init", &self.init)
            .field("filled", &self.filled)
            .field("capacity", &self.capacity)
            .finish()
    }
}

/// Creates a new `BorrowedBuf` from a fully initialized slice.
impl<'data, T: Copy> From<&'data mut [T]> for BorrowedBuf<'data, T> {
    #[inline]
    fn from(slice: &'data mut [T]) -> BorrowedBuf<'data, T> {
        BorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(slice as *mut [T] as *mut [MaybeUninit<T>]) },
            filled: 0,
            init: true,
        }
    }
}

/// Creates a new `BorrowedBuf` from an uninitialized buffer.
impl<'data, T: Copy> From<&'data mut [MaybeUninit<T>]> for BorrowedBuf<'data, T> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<T>]) -> BorrowedBuf<'data, T> {
        BorrowedBuf {
            buf,
            filled: 0,
            init: false,
        }
    }
}

/// Creates a new `BorrowedBuf` from a fully initialized array.
impl<'data, T: Copy, const N: usize> From<&'data mut [T; N]> for BorrowedBuf<'data, T> {
    #[inline]
    fn from(array: &'data mut [T; N]) -> BorrowedBuf<'data, T> {
        BorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(array as *mut [T] as *mut [MaybeUninit<T>]) },
            filled: 0,
            init: true,
        }
    }
}

/// Creates a new `BorrowedBuf` from an uninitialized buffer array.
impl<'data, T: Copy, const N: usize> From<&'data mut [MaybeUninit<T>; N]>
    for BorrowedBuf<'data, T>
{
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<T>; N]) -> BorrowedBuf<'data, T> {
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
impl<'data, T: Copy> From<BorrowedCursor<'data, T>> for BorrowedBuf<'data, T> {
    #[inline]
    fn from(buf: BorrowedCursor<'data, T>) -> BorrowedBuf<'data, T> {
        let filled = buf.filled();
        let init = buf.is_buf_init();
        let len = buf.buf_len();
        BorrowedBuf {
            // SAFETY: no initialized element is ever uninitialized as per `BorrowedBuf`'s
            // invariant, and the cursor holds the unique access to those elements for `'data`
            buf: unsafe { slice::from_raw_parts_mut(buf.buf.as_ptr().add(filled), len - filled) },
            filled: 0,
            init,
        }
    }
}

impl<'data, T> BorrowedBuf<'data, T> {
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
}

impl<'data, T: Copy> BorrowedBuf<'data, T> {
    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a shared reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled(self) -> &'data [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled_mut(self) -> &'data mut [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a cursor over the unfilled part of the buffer.
    #[inline]
    pub const fn unfilled<'this>(&'this mut self) -> BorrowedCursor<'this, T> {
        let borrowed_buf = NonNull::from_mut(self);
        BorrowedCursor {
            buf: NonNull::from_mut(self.buf).cast(),
            borrowed_buf,
        }
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The contents of the buffer are not modified.
    #[inline]
    pub const fn clear(&mut self) -> &mut Self {
        self.filled = 0;
        self
    }

    /// Asserts that the unfilled part of the buffer is initialized.
    ///
    /// # Safety
    ///
    /// All the bytes of the buffer must be initialized.
    #[inline]
    pub const unsafe fn set_init(&mut self) -> &mut Self {
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
pub struct BorrowedCursor<'a, T> {
    /// The start of the elements of the buffer this cursor was created from.
    /// Safety invariant: this points to the start of the *whole* buffer of `*borrowed_buf` and is
    /// valid for reads and writes of `(*borrowed_buf).buf.len()` elements, so that
    /// `(*borrowed_buf).filled` indexes into it.
    buf: NonNull<MaybeUninit<T>>,
    /// The buffer this cursor was created from.
    /// Safety invariants:
    /// 1. `(*borrowed_buf).buf` is *never* accessed by the owner of the pointee while the `buf`
    ///    field above is alive, because there is a `&mut` of the pointee while the cursor is
    ///    alive.
    /// 2. We promise to only access the `filled` and `init` fields and the metadata of the `buf`
    ///    field through the `borrowed_buf` pointer, never triggering any retag of `buf`'s pointer,
    ///    as the `buf` field above holds a reborrow of it and reaching the parent again would be a
    ///    foreign access for that reborrow. This includes not making a reference to the whole
    ///    pointee out of `borrowed_buf`, but only accessing those fields directly through pointer
    ///    manipulation.
    borrowed_buf: NonNull<BorrowedBuf<'a, T>>,
}

impl<T> Debug for BorrowedCursor<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let buf = BorrowedBufDebug {
            init: self.is_buf_init(),
            filled: self.filled(),
            capacity: self.buf_len(),
        };

        f.debug_struct("BorrowedCursor").field("buf", &buf).finish()
    }
}

// Helpers to access underlying buffer state.
impl<'a, T> BorrowedCursor<'a, T> {
    #[inline]
    const fn buf_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let len = self.buf_len();
        // SAFETY: `buf` points to `len` elements that this cursor borrows exclusively.
        unsafe { slice::from_raw_parts_mut(self.buf.as_ptr(), len) }
    }

    #[inline]
    const fn buf_len(&self) -> usize {
        // SAFETY: We read just the metadata of `buf` and avoid retagging the reference.
        unsafe {
            let borrowed_buf = self.borrowed_buf.as_ptr();
            let buf_ptr: *const &'a mut [MaybeUninit<T>] = &raw const (*borrowed_buf).buf;
            // Same layout:
            // https://doc.rust-lang.org/reference/type-layout.html#r-layout.pointer.intro
            let buf_ptr: *const *const [MaybeUninit<T>] = buf_ptr.cast();
            let buf: *const [MaybeUninit<T>] = *buf_ptr;
            buf.len()
        }
    }

    #[inline]
    fn unfilled_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let filled = self.filled();
        // SAFETY: always in bounds
        unsafe { self.buf_mut().get_unchecked_mut(filled..) }
    }

    #[inline]
    const fn filled(&self) -> usize {
        // SAFETY: We access just `filled` and avoid foreign read on `buf`.
        unsafe { (*self.borrowed_buf.as_ptr()).filled }
    }

    #[inline]
    const fn is_buf_init(&self) -> bool {
        // SAFETY: We access just `init` and avoid foreign read on `buf`.
        unsafe { (*self.borrowed_buf.as_ptr()).init }
    }

    /// # Safety
    ///
    /// In case of `true` all the elements of the cursor must be initialized.
    #[inline]
    const unsafe fn set_buf_init(&mut self, init: bool) {
        // SAFETY: We access just `init` and avoid foreign read on `buf`.
        unsafe {
            (*self.borrowed_buf.as_ptr()).init = init;
        }
    }

    /// # Safety
    ///
    /// The next `n` elements of the cursor must be initialized.
    #[inline]
    const unsafe fn add_filled(&mut self, n: usize) {
        // SAFETY: We access just `filled` and avoid foreign read on `buf`.
        unsafe {
            (*self.borrowed_buf.as_ptr()).filled += n;
        }
    }
}

impl<'a, T: Copy> BorrowedCursor<'a, T> {
    /// Reborrows this cursor by cloning it with a smaller lifetime.
    ///
    /// Since a cursor maintains unique access to its underlying buffer, the borrowed cursor is
    /// not accessible while the new cursor exists.
    #[inline]
    pub const fn reborrow<'this>(&'this mut self) -> BorrowedCursor<'this, T> {
        BorrowedCursor {
            buf: self.buf,
            borrowed_buf: self.borrowed_buf,
        }
    }

    /// Returns the available space in the cursor.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.buf_len() - self.filled()
    }

    /// Returns the number of bytes written to the `BorrowedBuf` this cursor was created from.
    ///
    /// In particular, the count returned is shared by all reborrows of the cursor.
    #[inline]
    pub const fn written(&self) -> usize {
        self.filled()
    }

    /// Returns `true` if the buffer is initialized.
    #[inline]
    pub const fn is_init(&self) -> bool {
        self.is_buf_init()
    }

    /// Set the buffer as fully initialized.
    ///
    /// # Safety
    ///
    /// All the bytes of the cursor must be initialized.
    #[inline]
    pub const unsafe fn set_init(&mut self) {
        // SAFETY: the caller guarantees that all the elements of the cursor are initialized.
        unsafe { self.set_buf_init(true) }
    }

    /// Returns a mutable reference to the whole cursor.
    ///
    /// # Safety
    ///
    /// The caller must not uninitialize any bytes of the cursor if it is initialized.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.unfilled_slice()
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
    pub const fn advance_checked(&mut self, n: usize) -> &mut Self {
        // The subtraction cannot underflow by invariant of this type.
        let init_unfilled = if self.is_buf_init() {
            self.buf_len() - self.filled()
        } else {
            0
        };
        assert!(n <= init_unfilled);

        // SAFETY: the next `n` elements are initialized, as asserted above.
        unsafe { self.advance(n) };
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
    pub const unsafe fn advance(&mut self, n: usize) -> &mut Self {
        // SAFETY: the caller guarantees that the first `n` elements of the cursor are initialized.
        unsafe { self.add_filled(n) };
        self
    }

    /// Appends data to the cursor, advancing position within its buffer.
    ///
    /// # Panics
    ///
    /// Panics if `self.capacity()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[T]) {
        assert!(self.capacity() >= buf.len());

        // SAFETY: we do not de-initialize any of the elements of the slice
        unsafe {
            self.as_mut()[..buf.len()].write_copy_of_slice(buf);
        }

        // SAFETY: these elements have just been initialized.
        unsafe { self.advance(buf.len()) };
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
    pub fn with_unfilled_buf<R>(&mut self, f: impl FnOnce(&mut BorrowedBuf<'_, T>) -> R) -> R {
        let mut buf = BorrowedBuf::from(self.reborrow());
        let prev_ptr = buf.buf as *const _;
        let res = f(&mut buf);

        // Check that the caller didn't replace the `BorrowedBuf`.
        // This is necessary for the safety of the code below: if the check wasn't
        // there, one could mark some elements as initialized even though they aren't.
        assert!(core::ptr::eq(prev_ptr, buf.buf));

        let filled = buf.filled;
        let init = buf.init;

        // Update `init` and `filled` fields with what was written to the buffer.
        // `self.buf.filled` was the starting length of the `BorrowedBuf`.
        //
        // SAFETY: These elements were initialized/filled in the `BorrowedBuf`, and therefore they
        // are initialized/filled in the cursor too, because the buffer wasn't replaced.
        unsafe {
            self.set_buf_init(init);
            self.advance(filled);
        }

        res
    }
}

impl<'a, T: Default + Copy> BorrowedCursor<'a, T> {
    /// Initializes all elements in the cursor with their default value and
    /// returns them.
    #[inline]
    pub fn ensure_init(&mut self) -> &mut [T] {
        if !self.is_init() {
            cfg_select! {
                all(feature = "nightly", feature = "__write_default") => {
                    self.unfilled_slice().write_default()
                },
                all(feature = "nightly", not(feature = "__write_default")) => {
                    maybeuninit_write_default(self.unfilled_slice())
                },
                _ => {
                    self.unfilled_slice().fill(MaybeUninit::new(Default::default()));
                },
            };

            unsafe { self.set_buf_init(true) };
        }

        // SAFETY: these elements have just been initialized if they weren't before
        unsafe { self.unfilled_slice().assume_init_mut() }
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
pub struct TrackingBorrowedBuf<'data, T> {
    /// The buffer's underlying data.
    buf: &'data mut [MaybeUninit<T>],
    /// The length of `self.buf` which is known to be filled.
    filled: usize,
    /// The length of `self.buf` which is known to be initialized.
    init: usize,
}

impl<T> fmt::Debug for TrackingBorrowedBuf<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        TrackingBorrowedBufDebug {
            init: self.init,
            filled: self.filled,
            capacity: self.capacity(),
        }
        .fmt(f)
    }
}

struct TrackingBorrowedBufDebug {
    init: usize,
    filled: usize,
    capacity: usize,
}

impl Debug for TrackingBorrowedBufDebug {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedBufTracking")
            .field("init", &self.init)
            .field("filled", &self.filled)
            .field("capacity", &self.capacity)
            .finish()
    }
}

/// Create a new `TrackingBorrowedBuf` from a fully initialized slice.
impl<'data, T: Copy> From<&'data mut [T]> for TrackingBorrowedBuf<'data, T> {
    #[inline]
    fn from(slice: &'data mut [T]) -> TrackingBorrowedBuf<'data, T> {
        let len = slice.len();

        TrackingBorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            // buf: unsafe { (slice as *mut [u8]).as_uninit_slice_mut().unwrap() },
            buf: unsafe {
                core::slice::from_raw_parts_mut((slice as *mut [T]) as *mut MaybeUninit<T>, len)
            },
            filled: 0,
            init: len,
        }
    }
}

/// Create a new `TrackingBorrowedBuf` from an uninitialized buffer.
///
/// Use `set_init` if part of the buffer is known to be already initialized.
impl<'data, T: Copy> From<&'data mut [MaybeUninit<T>]> for TrackingBorrowedBuf<'data, T> {
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<T>]) -> TrackingBorrowedBuf<'data, T> {
        TrackingBorrowedBuf {
            buf,
            filled: 0,
            init: 0,
        }
    }
}

/// Creates a new `TrackingBorrowedBuf` from a fully initialized array.
impl<'data, T: Copy, const N: usize> From<&'data mut [T; N]> for TrackingBorrowedBuf<'data, T> {
    #[inline]
    fn from(array: &'data mut [T; N]) -> TrackingBorrowedBuf<'data, T> {
        TrackingBorrowedBuf {
            // SAFETY: initialized data never becoming uninitialized is an invariant of BorrowedBuf
            buf: unsafe { &mut *(array as *mut [T] as *mut [MaybeUninit<T>]) },
            filled: 0,
            init: N,
        }
    }
}

/// Creates a new `TrackingBorrowedBuf` from an uninitialized buffer array.
impl<'data, T: Copy, const N: usize> From<&'data mut [MaybeUninit<T>; N]>
    for TrackingBorrowedBuf<'data, T>
{
    #[inline]
    fn from(buf: &'data mut [MaybeUninit<T>; N]) -> TrackingBorrowedBuf<'data, T> {
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
impl<'data, T: Copy> From<TrackingBorrowedCursor<'data, T>> for TrackingBorrowedBuf<'data, T> {
    #[inline]
    fn from(buf: TrackingBorrowedCursor<'data, T>) -> TrackingBorrowedBuf<'data, T> {
        let filled = buf.filled();
        let init = buf.init();
        let len = buf.buf_len();
        TrackingBorrowedBuf {
            // SAFETY: no initialized element is ever uninitialized as per `BorrowedBuf`'s
            // invariant, and the cursor holds the unique access to those elements for `'data`
            buf: unsafe { slice::from_raw_parts_mut(buf.buf.as_ptr().add(filled), len - filled) },
            filled: 0,
            init,
        }
    }
}

impl<'data, T> TrackingBorrowedBuf<'data, T> {
    /// Returns the total capacity of the buffer.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Returns the length of the filled part of the buffer.
    #[inline]
    pub const fn len(&self) -> usize {
        self.filled
    }

    /// Returns `true` if the buffer is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Returns the length of the initialized part of the buffer.
    #[inline]
    pub const fn init_len(&self) -> usize {
        self.init
    }
}

impl<'data, T: Copy> TrackingBorrowedBuf<'data, T> {
    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a shared reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled(self) -> &'data [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked(..self.filled);
            buf.assume_init_ref()
        }
    }

    /// Returns a mutable reference to the filled portion of the buffer with its original lifetime.
    #[inline]
    pub fn into_filled_mut(self) -> &'data mut [T] {
        // SAFETY: We only slice the filled part of the buffer, which is always valid
        unsafe {
            let buf = self.buf.get_unchecked_mut(..self.filled);
            buf.assume_init_mut()
        }
    }

    /// Returns a cursor over the unfilled part of the buffer.
    #[inline]
    pub const fn unfilled<'this>(&'this mut self) -> TrackingBorrowedCursor<'this, T> {
        let borrowed_buf = NonNull::from_mut(self);
        TrackingBorrowedCursor {
            buf: NonNull::from_mut(self.buf).cast(),
            borrowed_buf,
            start: self.filled,
        }
    }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The number of initialized bytes is not changed, and the contents of the buffer are not
    /// modified.
    #[inline]
    pub const fn clear(&mut self) -> &mut Self {
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
    pub const unsafe fn set_init(&mut self, n: usize) -> &mut Self {
        self.init = max(self.init, n);
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
pub struct TrackingBorrowedCursor<'a, T> {
    /// The start of the elements of the buffer this cursor was created from.
    /// Safety invariant: this points to the start of the *whole* buffer of `*borrowed_buf` and is
    /// valid for reads and writes of `(*borrowed_buf).buf.len()` elements, so that
    /// `(*borrowed_buf).filled` indexes into it.
    buf: NonNull<MaybeUninit<T>>,
    /// The buffer this cursor was created from.
    /// Safety invariants:
    /// 1. `(*borrowed_buf).buf` is *never* accessed by the owner of the pointee while the `buf`
    ///    field above is alive, because there is a `&mut` of the pointee while the cursor is
    ///    alive.
    /// 2. We promise to only access the `filled` and `init` fields and the metadata of the `buf`
    ///    field through the `borrowed_buf` pointer, never triggering any retag of `buf`'s pointer,
    ///    as the `buf` field above holds a reborrow of it and reaching the parent again would be a
    ///    foreign access for that reborrow. This includes not making a reference to the whole
    ///    pointee out of `borrowed_buf`, but only accessing those fields directly through pointer
    ///    manipulation.
    borrowed_buf: NonNull<TrackingBorrowedBuf<'a, T>>,
    /// The length of the filled portion of the underlying buffer at the time of the cursor's
    /// creation.
    start: usize,
}

impl<T> Debug for TrackingBorrowedCursor<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let buf = TrackingBorrowedBufDebug {
            init: self.init(),
            filled: self.filled(),
            capacity: self.buf_len(),
        };

        f.debug_struct("BorrowedCursor").field("buf", &buf).finish()
    }
}

// Helpers to access underlying buffer state.
impl<'a, T> TrackingBorrowedCursor<'a, T> {
    #[inline]
    const fn buf(&self) -> &[MaybeUninit<T>] {
        let len = self.buf_len();
        // SAFETY: `buf` points to `len` elements that this cursor borrows exclusively.
        unsafe { slice::from_raw_parts(self.buf.as_ptr().cast_const(), len) }
    }

    #[inline]
    const fn buf_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let len = self.buf_len();
        // SAFETY: `buf` points to `len` elements that this cursor borrows exclusively.
        unsafe { slice::from_raw_parts_mut(self.buf.as_ptr(), len) }
    }

    #[inline]
    const fn buf_len(&self) -> usize {
        // SAFETY: We read just the metadata of `buf` and avoid retagging the reference.
        unsafe {
            let borrowed_buf = self.borrowed_buf.as_ptr();
            let buf_ptr: *const &'a mut [MaybeUninit<T>] = &raw const (*borrowed_buf).buf;
            // Same layout:
            // https://doc.rust-lang.org/reference/type-layout.html#r-layout.pointer.intro
            let buf_ptr: *const *const [MaybeUninit<T>] = buf_ptr.cast();
            let buf: *const [MaybeUninit<T>] = *buf_ptr;
            buf.len()
        }
    }

    #[inline]
    fn unfilled_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let filled = self.filled();
        // SAFETY: always in bounds
        unsafe { self.buf_mut().get_unchecked_mut(filled..) }
    }

    #[inline]
    fn init_slice_ref(&self) -> &[MaybeUninit<T>] {
        let filled = self.filled();
        let init = self.init();
        // SAFETY: always in bounds
        unsafe { self.buf().get_unchecked(filled..init) }
    }

    #[inline]
    fn init_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let filled = self.filled();
        let init = self.init();
        // SAFETY: always in bounds
        unsafe { self.buf_mut().get_unchecked_mut(filled..init) }
    }

    #[inline]
    fn uninit_slice(&mut self) -> &mut [MaybeUninit<T>] {
        let init = self.init();
        // SAFETY: always in bounds
        unsafe { self.buf_mut().get_unchecked_mut(init..) }
    }

    #[inline]
    const fn filled(&self) -> usize {
        // SAFETY: We access just `filled` and avoid foreign read on `buf`.
        unsafe { (*self.borrowed_buf.as_ptr()).filled }
    }

    #[inline]
    const fn init(&self) -> usize {
        // SAFETY: We access just `init` and avoid foreign read on `buf`.
        unsafe { (*self.borrowed_buf.as_ptr()).init }
    }

    /// # Safety
    ///
    /// In case of `true` all the elements of the cursor must be initialized.
    #[inline]
    const unsafe fn set_buf_init(&mut self, init: usize) {
        // SAFETY: We access just `init` and avoid foreign read on `buf`.
        unsafe {
            (*self.borrowed_buf.as_ptr()).init = init;
        }
    }

    /// # Safety
    ///
    /// The next `n` elements of the cursor must be initialized.
    #[inline]
    const unsafe fn add_filled(&mut self, n: usize) {
        // SAFETY: We access just `filled` and avoid foreign read on `buf`.
        unsafe {
            (*self.borrowed_buf.as_ptr()).filled += n;
        }
    }
}

impl<'a, T: Copy> TrackingBorrowedCursor<'a, T> {
    /// Reborrows this cursor by cloning it with a smaller lifetime.
    ///
    /// Since a cursor maintains unique access to its underlying buffer, the borrowed cursor is
    /// not accessible while the new cursor exists.
    #[inline]
    pub const fn reborrow<'this>(&'this mut self) -> TrackingBorrowedCursor<'this, T> {
        TrackingBorrowedCursor {
            buf: self.buf,
            borrowed_buf: self.borrowed_buf,
            start: self.start,
        }
    }

    /// Returns the available space in the cursor.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.buf_len() - self.filled()
    }

    /// Returns the number of bytes written to this cursor since it was created from a
    /// `BorrowedBuf`.
    ///
    /// Note that if this cursor is a reborrowed clone of another, then the count returned is the
    /// count written via either cursor, not the count since the cursor was reborrowed.
    #[inline]
    pub const fn written(&self) -> usize {
        self.filled() - self.start
    }

    /// Returns a shared reference to the initialized portion of the cursor.
    #[inline]
    pub fn init_ref(&self) -> &[T] {
        let buf = self.init_slice_ref();
        // SAFETY: We only slice the initialized part of the buffer, which is always valid
        unsafe { buf.assume_init_ref() }
    }

    /// Returns a mutable reference to the initialized portion of the cursor.
    #[inline]
    pub fn init_mut(&mut self) -> &mut [T] {
        let buf = self.init_slice();
        // SAFETY: We only slice the initialized part of the buffer, which is always valid
        unsafe { buf.assume_init_mut() }
    }

    /// Returns a mutable reference to the uninitialized part of the cursor.
    ///
    /// It is safe to uninitialize any of these bytes.
    #[inline]
    pub fn uninit_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.uninit_slice()
    }

    /// Returns a mutable reference to the whole cursor.
    ///
    /// # Safety
    ///
    /// The caller must not uninitialize any bytes in the initialized portion of the cursor.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.unfilled_slice()
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
    pub const fn advance_checked(&mut self, n: usize) -> &mut Self {
        let filled = self.filled().strict_add(n);
        let init = self.init();
        assert!(filled <= init);

        // SAFETY: the next `n` elements are initialized, as asserted above.
        unsafe { self.advance(n) };
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
    pub const unsafe fn advance(&mut self, n: usize) -> &mut Self {
        // SAFETY: the caller guarantees that the first `n` elements of the cursor are initialized.
        unsafe {
            self.add_filled(n);
            self.set_buf_init(max(self.init(), self.filled()));
        }
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
        unsafe { self.set_buf_init(cmp::max(self.init(), self.filled() + n)) };
        self
    }

    /// Appends data to the cursor, advancing position within its buffer.
    ///
    /// # Panics
    ///
    /// Panics if `self.capacity()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[T]) {
        assert!(self.capacity() >= buf.len());

        // SAFETY: we do not de-initialize any of the elements of the slice
        unsafe { self.as_mut()[..buf.len()].write_copy_of_slice(buf) };

        // SAFETY: We just added the entire contents of buf to the filled section.
        unsafe {
            self.set_init(buf.len());
            self.advance(buf.len());
        }
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
    pub fn with_unfilled_buf<R>(
        &mut self, f: impl FnOnce(&mut TrackingBorrowedBuf<'_, T>) -> R,
    ) -> R {
        let mut buf = TrackingBorrowedBuf::from(self.reborrow());
        let prev_ptr = buf.buf as *const _;
        let res = f(&mut buf);

        // Check that the caller didn't replace the `BorrowedBuf`.
        // This is necessary for the safety of the code below: if the check wasn't
        // there, one could mark some elements as initialized even though they aren't.
        assert!(core::ptr::eq(prev_ptr, buf.buf));

        let filled = buf.filled;
        let init = buf.init;

        // Update `init` and `filled` fields with what was written to the buffer.
        // `self.buf.filled` was the starting length of the `BorrowedBuf`.
        //
        // SAFETY: These elements were initialized/filled in the `BorrowedBuf`, and therefore they
        // are initialized/filled in the cursor too, because the buffer wasn't replaced.
        unsafe {
            self.set_buf_init(init);
            self.advance(filled);
        }

        res
    }
}

impl<'a, T: Default + Copy> TrackingBorrowedCursor<'a, T> {
    /// Initializes all elements in the cursor with their default value and
    /// returns them.
    ///
    /// This includes items not filled and not initialized.
    #[inline]
    pub fn ensure_init(&'a mut self) -> &'a mut [T] {
        let capacity = self.capacity();

        if self.init() > 0 {
            // SAFETY: always in bounds and we never uninitialize these elements.
            let unfilled = self.unfilled_slice();

            cfg_select! {
                all(feature = "nightly", feature = "__write_default") => unfilled.write_default(),
                all(feature = "nightly", not(feature = "__write_default")) => {
                    maybeuninit_write_default(unfilled)
                },
                _ => {
                    unfilled.fill(MaybeUninit::new(Default::default()));
                },
            };
        }

        // SAFETY: these elements have just been initialized if they weren't before
        unsafe {
            self.set_buf_init(capacity);
            self.unfilled_slice().assume_init_mut()
        }
    }
}

#[cfg(feature = "nightly")]
fn maybeuninit_write_default<T>(buf: &mut [MaybeUninit<T>]) -> &mut [T]
where
    T: Default,
{
    trait DefaultSpec: Default {
        fn write_default(buf: &mut [MaybeUninit<Self>]) -> &mut [Self];
    }

    impl<T: Default> DefaultSpec for T {
        default fn write_default(buf: &mut [MaybeUninit<Self>]) -> &mut [Self] {
            buf.write_with(|_| T::default())
        }
    }

    macro_rules! spec_default_zero {
        ($ty:ty) => {
            impl DefaultSpec for $ty {
                fn write_default(buf: &mut [MaybeUninit<Self>]) -> &mut [Self] {
                    // SAFETY:
                    // `Default::default` is equivalent to zero-initialization
                    // for all these types, and this initializes the entire
                    // slice.
                    unsafe {
                        buf.as_mut_ptr().write_bytes(0, buf.len());
                        buf.assume_init_mut()
                    }
                }
            }
        };
    }

    spec_default_zero!(i8);
    spec_default_zero!(u8);
    spec_default_zero!(i16);
    spec_default_zero!(u16);
    spec_default_zero!(i32);
    spec_default_zero!(u32);
    spec_default_zero!(i64);
    spec_default_zero!(u64);
    spec_default_zero!(i128);
    spec_default_zero!(u128);
    spec_default_zero!(isize);
    spec_default_zero!(usize);

    T::write_default(buf)
}

const fn max(this: usize, other: usize) -> usize {
    if other < this {
        this
    } else {
        other
    }
}
