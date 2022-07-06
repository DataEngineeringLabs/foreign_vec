//! This library offers [`ForeignVec`], a zero-cost abstraction to store either [`Vec`]
//! or an immutable region allocated by an external allocator.
//!
//! The primary use-case of this library is when you have an in-memory format
//! that has the same in-memory representation in both Rust and other languages
//! and you have a specification to share this across language boundaries at zero cost,
//! via FFI.
//!
//! The typical approach is via bindgen and some semantics on how memory is released.
//!
//! [`ForeignVec`] is exposes a small struct that
//! behaves either as a `Vec` (allocated by Rust), or as `(ptr, len, owner)` allocated by
//! the external allocator, via a zero-cost implementation of [`core::ops::Deref<T>`].
#![deny(missing_docs)]
#![no_std]

extern crate alloc;

use core::mem::ManuallyDrop;
use core::ops::DerefMut;

use alloc::vec::Vec;

/// Mode of deallocating memory regions
enum Allocation<D> {
    /// Native allocation
    Native,
    // A foreign allocator and its ref count
    Foreign(D),
}

/// A continuous memory region that may be allocated externally.
///
/// In the most common case, this is created from [`Vec`].
/// However, this region may also be allocated by a foreign allocator `D`
/// and behave as `&[T]`.
pub struct ForeignVec<D, T> {
    /// An implementation using an `enum` of a `Vec` or a foreign pointer is not used
    /// because `deref` is at least 50% more expensive than the deref of a `Vec`.
    data: ManuallyDrop<Vec<T>>,
    /// the region was allocated
    allocation: Allocation<D>,
}

impl<D, T> ForeignVec<D, T> {
    /// Takes ownership of an allocated memory region `[ptr, ptr+len[`,
    /// # Panic
    /// This function panics if `ptr` is null
    /// # Safety
    /// This function is safe iff:
    /// * the region is properly allocated in that a slice can be safely built from it.
    /// * the region is immutable.
    /// # Implementation
    /// This function leaks iff `owner` does not deallocate the region when dropped.
    #[inline]
    pub unsafe fn from_owned(ptr: *const T, len: usize, owner: D) -> Self {
        assert!(!ptr.is_null());

        // This line is technically outside the assumptions of `Vec::from_raw_parts`, since
        // `ptr` was not allocated by `Vec`. However, one of the invariants of this struct
        // is that we do never expose this region as a `Vec`; we only use `Vec` on it to provide
        // immutable access to the region (via `Vec::deref` to `&[T]`).
        let data = Vec::from_raw_parts(ptr as *mut T, len, len);
        let data = ManuallyDrop::new(data);

        Self {
            data,
            allocation: Allocation::Foreign(owner),
        }
    }

    /// Returns a `Some` mutable reference of [`Vec<T>`] iff this was initialized
    /// from a [`Vec<T>`] and `None` otherwise.
    pub fn get_vec(&mut self) -> Option<&mut Vec<T>> {
        match &self.allocation {
            Allocation::Foreign(_) => None,
            Allocation::Native => Some(self.data.deref_mut()),
        }
    }
}

impl<D, T> Drop for ForeignVec<D, T> {
    #[inline]
    fn drop(&mut self) {
        match self.allocation {
            Allocation::Foreign(_) => {
                // the foreign is dropped via its `Drop`
            }
            Allocation::Native => {
                let data = core::mem::take(&mut self.data);
                let _ = ManuallyDrop::into_inner(data);
            }
        }
    }
}

impl<D, T> core::ops::Deref for ForeignVec<D, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        &self.data
    }
}

impl<T: core::fmt::Debug, D> core::fmt::Debug for ForeignVec<D, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<D, T> From<Vec<T>> for ForeignVec<D, T> {
    #[inline]
    fn from(data: Vec<T>) -> Self {
        Self {
            data: ManuallyDrop::new(data),
            allocation: Allocation::Native,
        }
    }
}
