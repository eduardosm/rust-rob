// Copyright 2018 Eduardo Sánchez Muñoz
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! This crate provides `Rob<'a, T>` a type that can contain either a
//! borrwed reference or an owned `Box`. It is similar to `std::borrow::Cow<'a, T>`,
//! but it always uses a `Box` to stored owned values.
//!
//! The main difference with `Cow` is that `Rob` is not implemented as
//! an enum, instead it is a struct with a pointer and a flag that
//! indicates whether the value is owned or not. This allows to use
//! the value by accessing directly the pointer, without the overhead
//! of matching an enum needed by `Cow`.

#[cfg(test)]
mod tests;

use std::ptr::NonNull;
use std::marker::PhantomData;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// The `Rob` type. See the crate documentation.
pub struct Rob<'a, T: 'a + ?Sized> {
    ptr: NonNull<T>,
    is_owned: bool,
    marker1: PhantomData<&'a T>,
    marker2: PhantomData<T>,
}

unsafe impl<'a, T: 'a + ?Sized> Send for Rob<'a, T>
where
    T: Send + Sync
{}

unsafe impl<'a, T: 'a + ?Sized> Sync for Rob<'a, T>
where
    T: Sync
{}

impl<'a, T: 'a + ?Sized> Drop for Rob<'a, T> {
    fn drop(&mut self) {
        if self.is_owned {
            unsafe { Box::from_raw(self.ptr.as_ptr()) };
        }
    }
}

impl<'a, T: 'a> Rob<'a, T> {
    /// Creates a new `Rob` with an owned value.
    ///
    /// Example
    /// -------
    /// ```
    /// let x = rob::Rob::from_value(123i32);
    /// assert_eq!(*x, 123);
    /// assert!(rob::Rob::is_owned(&x));
    /// ```
    #[inline]
    pub fn from_value(value: T) -> Self {
        Self::from_box(Box::new(value))
    }
}

impl<'a, T: 'a + ?Sized> Rob<'a, T> {
    /// Creates a new `Rob` with a borrowed reference.
    ///
    /// Example
    /// -------
    /// ```
    /// let value = 123i32;
    /// let x = rob::Rob::from_ref(&value);
    /// assert_eq!(*x, 123);
    /// assert!(!rob::Rob::is_owned(&x));
    /// ```
    #[inline]
    pub const fn from_ref(r: &'a T) -> Self {
        Self {
            // This is equivalent to `NonNull::from(r)`, which can't be used
            // in `const fn`.
            ptr: unsafe { NonNull::new_unchecked(r as *const _ as *mut _) },
            is_owned: false,
            marker1: PhantomData,
            marker2: PhantomData,
        }
    }
    
    /// Creates a new `Rob` with an owned value that is already boxed.
    ///
    /// Example
    /// -------
    /// ```
    /// let x = rob::Rob::from_box(Box::new(123i32));
    /// assert_eq!(*x, 123);
    /// assert!(rob::Rob::is_owned(&x));
    /// ```
    #[inline]
    pub fn from_box(b: Box<T>) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(b)) },
            is_owned: true,
            marker1: PhantomData,
            marker2: PhantomData,
        }
    }
    
    /// Creates a new `Rob` from a raw pointer and an owned flag. If
    /// `is_owned` is `true`, `ptr` should come from `Box::into_raw`.
    #[inline]
    pub const unsafe fn from_raw(ptr: *mut T, is_owned: bool) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            is_owned: is_owned,
            marker1: PhantomData,
            marker2: PhantomData,
        }
    }
    
    /// Consumes `this`, returning a raw pointer to the value and a
    /// flag indicating whether the values is owned or not.
    #[inline]
    pub fn into_raw(this: Self) -> (*mut T, bool) {
        let ptr = this.ptr.as_ptr();
        let is_owned = this.is_owned;
        std::mem::forget(this);
        (ptr, is_owned)
    }
    
    /// If the value is not owned, returns a reference to it with
    /// lifetime `'a`.
    #[inline]
    pub fn as_ref(this: &Self) -> Option<&'a T> {
        if !this.is_owned {
            unsafe { Some(&*this.ptr.as_ptr()) }
        } else {
            None
        }
    }
    
    /// Returns whether the value is owned or not.
    #[inline]
    pub const fn is_owned(this: &Self) -> bool {
        this.is_owned
    }
}

impl<'a, T: 'a + ?Sized> Rob<'a, T>
    where T: std::borrow::ToOwned,
          <T as std::borrow::ToOwned>::Owned: Into<Box<T>>
{
    /// Consumes `this`, returning a `Box` containing the value, cloning
    /// it if it was not owned.
    pub fn into_box(this: Self) -> Box<T> {
        if this.is_owned {
            let ptr = this.ptr.as_ptr();
            std::mem::forget(this);
            unsafe { Box::from_raw(ptr) }
        } else {
            this.to_owned().into()
        }
    }
    
    /// Returns a mutable reference to the value, cloning it if it was
    /// not owned.
    pub fn to_mut(this: &mut Self) -> &mut T {
        unsafe {
            if !this.is_owned {
                let b: Box<T> = this.to_owned().into();
                this.ptr = NonNull::new_unchecked(Box::into_raw(b));
                this.is_owned = true;
            }
            
            &mut *this.ptr.as_mut()
        }
    }
}

impl<'a, T: 'a> From<T> for Rob<'a, T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::from_value(value)
    }
}

impl<'a, T: 'a + ?Sized> From<&'a T> for Rob<'a, T> {
    #[inline]
    fn from(r: &'a T) -> Self {
        Self::from_ref(r)
    }
}

impl<'a, T: 'a + ?Sized> From<Box<T>> for Rob<'a, T> {
    #[inline]
    fn from(b: Box<T>) -> Self {
        Self::from_box(b)
    }
}

impl<'a, T: 'a> From<Vec<T>> for Rob<'a, [T]> {
    #[inline]
    fn from(vec: Vec<T>) -> Self {
        Self::from_box(vec.into_boxed_slice())
    }
}

impl<'a> From<String> for Rob<'a, str> {
    #[inline]
    fn from(s: String) -> Self {
        Self::from_box(s.into_boxed_str())
    }
}

impl<'a> From<std::ffi::CString> for Rob<'a, std::ffi::CStr> {
    #[inline]
    fn from(s: std::ffi::CString) -> Self {
        Self::from_box(s.into_boxed_c_str())
    }
}

impl<'a> From<std::ffi::OsString> for Rob<'a, std::ffi::OsStr> {
    #[inline]
    fn from(s: std::ffi::OsString) -> Self {
        Self::from_box(s.into_boxed_os_str())
    }
}

impl<'a> From<std::path::PathBuf> for Rob<'a, std::path::Path> {
    #[inline]
    fn from(s: std::path::PathBuf) -> Self {
        Self::from_box(s.into_boxed_path())
    }
}

impl<'a, T> From<std::borrow::Cow<'a, T>> for Rob<'a, T>
    where T: std::borrow::ToOwned,
          <T as std::borrow::ToOwned>::Owned: Into<Box<T>>,
{
    fn from(cow: std::borrow::Cow<'a, T>) -> Self {
        match cow {
            std::borrow::Cow::Borrowed(r) => Self::from_ref(r),
            std::borrow::Cow::Owned(o) => Self::from_box(o.into()),
        }
    }
}

impl<'a, T: 'a + Clone> Clone for Rob<'a, T> {
    fn clone(&self) -> Self {
        if self.is_owned {
            Self::from_value((**self).clone())
        } else {
            unsafe {
                Self::from_ref(&*self.ptr.as_ptr())
            }
        }
    }
}

impl<'a, T: 'a + ?Sized + Debug> Debug for Rob<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as Debug>::fmt(&**self, f)
    }
}

impl<'a, T: 'a + ?Sized + Hash> Hash for Rob<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        <T as Hash>::hash(&**self, state)
    }
}

impl<'a, T: 'a + ?Sized + PartialEq> PartialEq for Rob<'a, T> {
    #[inline]
    fn eq(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialEq>::eq(&**self, &**other)
    }
    
    #[inline]
    fn ne(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialEq>::ne(&**self, &**other)
    }
}

impl<'a, T: 'a + ?Sized + PartialOrd> PartialOrd for Rob<'a, T> {
    #[inline]
    fn partial_cmp(&self, other: &Rob<'a, T>) -> Option<std::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(&**self, &**other)
    }
    
    #[inline]
    fn lt(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialOrd>::lt(&**self, &**other)
    }
    
    #[inline]
    fn le(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialOrd>::le(&**self, &**other)
    }
    
    #[inline]
    fn ge(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialOrd>::ge(&**self, &**other)
    }
    
    #[inline]
    fn gt(&self, other: &Rob<'a, T>) -> bool {
        <T as PartialOrd>::gt(&**self, &**other)
    }
}

impl<'a, T: 'a + ?Sized> std::ops::Deref for Rob<'a, T> {
    type Target = T;
    
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl<'a, T: 'a + ?Sized> std::borrow::Borrow<T> for Rob<'a, T> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<'a, T: 'a + ?Sized> AsRef<T> for Rob<'a, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}
