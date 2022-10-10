//! A write-only reference.
#![deny(
    missing_docs,
    clippy::all,
    clippy::cargo,
    clippy::missing_const_for_fn,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate
)]
#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};

/// A write-only reference.
#[repr(transparent)]
pub struct OutRef<'a, T: ?Sized> {
    data: NonNull<T>,
    _marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for OutRef<'_, T> {}
unsafe impl<T: Sync> Sync for OutRef<'_, T> {}

impl<'a, T: ?Sized> OutRef<'a, T> {
    /// Forms an [`OutRef<'a, T>`](OutRef)
    ///
    /// # Safety
    ///
    /// * `data` must be valid for writes.
    /// * `data` must be properly aligned.
    #[inline(always)]
    #[must_use]
    pub unsafe fn from_raw(data: *mut T) -> Self {
        Self {
            data: NonNull::new_unchecked(data),
            _marker: PhantomData,
        }
    }
}

impl<'a, T> OutRef<'a, T> {
    /// Forms an [`OutRef<'a, T>`](OutRef).
    #[inline(always)]
    #[must_use]
    pub fn from_mut(data: &'a mut T) -> Self
    where
        T: Copy,
    {
        unsafe { Self::from_raw(data) }
    }

    /// Forms an [`OutRef<'a, T>`](OutRef) from an uninitialized value.
    #[inline(always)]
    #[must_use]
    pub fn from_uninit(data: &'a mut MaybeUninit<T>) -> Self {
        let data: *mut T = MaybeUninit::as_mut_ptr(data);
        unsafe { Self::from_raw(data.cast()) }
    }

    /// Returns an unsafe mutable pointer to the value.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr().cast()
    }
}

impl<'a, T> OutRef<'a, [T]> {
    /// Forms an [`OutRef<'a, [T]>`](OutRef).
    #[inline(always)]
    #[must_use]
    pub fn from_slice(slice: &'a mut [T]) -> Self
    where
        T: Copy,
    {
        unsafe { Self::from_raw(slice) }
    }

    /// Forms an [`OutRef<'a, [T]>`](OutRef) from an uninitialized slice.
    #[inline(always)]
    #[must_use]
    pub fn from_uninit_slice(slice: &'a mut [MaybeUninit<T>]) -> Self {
        let slice: *mut [T] = {
            let len = slice.len();
            let data = slice.as_mut_ptr().cast();
            ptr::slice_from_raw_parts_mut(data, len)
        };
        unsafe { Self::from_raw(slice) }
    }

    /// Returns true if the slice has a length of 0.
    #[inline(always)]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements in the slice.
    #[inline(always)]
    #[must_use]
    pub const fn len(&self) -> usize {
        NonNull::len(self.data)
    }

    /// Returns an unsafe mutable pointer to the slice's buffer.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr().cast()
    }
}

#[cfg(test)]
mod tests {
    use super::OutRef;

    use core::{mem, ptr, slice};

    fn fill<T: Copy>(mut buf: OutRef<'_, [T]>, val: T) -> &'_ mut [T] {
        let len = buf.len();
        let data = buf.as_mut_ptr();
        unsafe {
            if len > 0 {
                if mem::size_of::<T>() == 0 {
                    // do nothing
                } else if mem::size_of::<T>() == 1 {
                    let val: u8 = mem::transmute_copy(&val);
                    data.write_bytes(val, len)
                } else {
                    data.write(val);

                    let mut n = 1;
                    while n <= len / 2 {
                        ptr::copy_nonoverlapping(data, data.add(n), n);
                        n *= 2;
                    }

                    let count = len - n;
                    if count > 0 {
                        ptr::copy_nonoverlapping(data, data.add(n), count);
                    }
                }
            }
            slice::from_raw_parts_mut(data, len)
        }
    }

    #[test]
    fn fill_vec() {
        for n in 0..128 {
            let mut v: Vec<u32> = Vec::with_capacity(n);
            let buf = OutRef::from_uninit_slice(v.spare_capacity_mut());
            fill(buf, 0x12345678);
            unsafe { v.set_len(n) };
            for &x in &v {
                assert_eq!(x, 0x12345678);
            }
            drop(v);
        }
    }
}
