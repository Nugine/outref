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
pub struct OutRef<'a, T: ?Sized> {
    data: NonNull<T>,
    _marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for OutRef<'_, T> {}
unsafe impl<T: Sync> Sync for OutRef<'_, T> {}

impl<'a, T: ?Sized> OutRef<'a, T> {
    /// Forms an [`OutRef<'a, T>`](OutRef).
    #[inline(always)]
    #[must_use]
    pub fn new(data: &'a mut T) -> Self {
        Self {
            data: NonNull::from(data),
            _marker: PhantomData,
        }
    }

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
    /// Forms an [`OutRef<'a, T>`](OutRef) from an uninitialized value.
    #[inline(always)]
    #[must_use]
    pub fn uninit(value: &'a mut MaybeUninit<T>) -> Self {
        let data: *mut T = MaybeUninit::as_mut_ptr(value);
        unsafe { Self::from_raw(data.cast()) }
    }

    /// Returns an unsafe mutable pointer to the value.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr().cast()
    }

    /// Sets the value of the [`OutRef<'a, T>`](OutRef).
    #[inline(always)]
    pub fn write(&mut self, val: T) -> &mut T {
        let p = self.as_mut_ptr();
        unsafe {
            p.write(val);
            &mut *p
        }
    }
}

impl<'a, T> OutRef<'a, [T]> {
    /// Forms an [`OutRef<'a, [T]>`](OutRef) from an uninitialized slice.
    #[inline(always)]
    #[must_use]
    pub fn uninit_slice(slice: &'a mut [MaybeUninit<T>]) -> Self {
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
                if mem::size_of::<T>() == 1 {
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

    fn vec_uninit_part<T>(v: &mut Vec<T>) -> OutRef<'_, [T]> {
        unsafe {
            let cap = v.capacity();
            let len = v.len();
            let data = v.as_mut_ptr().add(len);
            OutRef::from_raw(ptr::slice_from_raw_parts_mut(data, cap - len))
        }
    }

    #[test]
    fn fill_vec() {
        for n in 0..128 {
            let mut v: Vec<u32> = Vec::with_capacity(n);
            let buf = vec_uninit_part(&mut v);
            fill(buf, 0x12345678);
            unsafe { v.set_len(n) };
            for &x in &v {
                assert_eq!(x, 0x12345678);
            }
            drop(v);
        }
    }
}
