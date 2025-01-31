//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
A runtime-checked sending cell.

This verifies that all use of the resulting value occurs on the same thread.
*/

use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::thread::ThreadId;
use crate::unsafe_send_cell::UnsafeSendCell;

pub struct SendCell<T> {
    inner: Option<UnsafeSendCell<T>>,
    thread_id: ThreadId,
}

impl <T> SendCell<T> {
    /**
    Creates a new cell.

    This constructor wil "remember" the current thread.  Subsequent access
    will be checked against the constructed value.
*/
    #[inline]
    pub fn new(t: T) -> SendCell<T> {
        SendCell {
            //safe because drop is verified
            inner: Some(unsafe{UnsafeSendCell::new_unchecked(t)}),
            thread_id: crate::sys::thread::current().id(),
        }
    }

    /**
    Unsafely accesses the underlying value, without checking the accessing thread.
*/
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        &*self.inner.as_ref().expect("gone").get()
    }
    /**
    Access the underlying value.

    # Panics

    This function will panic if accessed from a different thread than the cell was created on.
*/
    #[inline]
    pub fn get(&self) -> &T {
        assert_eq!(self.thread_id, crate::sys::thread::current().id(), "Access SendCell from incorrect thread");
        //safe with assertion
        unsafe { self.get_unchecked() }
    }

    /**
    Unsafely accesses the underlying value, without checking the accessing thread.
*/
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        &mut *self.inner.as_mut().expect("gone").get_mut()
    }

    /**
    Accesses the underlying value.

    This function will panic if accessed from a different thread than the cell was created on.
*/
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        assert_eq!(self.thread_id, crate::sys::thread::current().id(), "Access SendCell from incorrect thread");
        unsafe { self.get_unchecked_mut()}
    }

    /**
    Unsafely accesses the underlying value, without checking the accessing thread.
    */
    #[inline]
    pub unsafe fn into_unchecked_inner(mut self)  -> T {
        self.inner.take().expect("gone").into_inner()
    }
    /**
    Accesses the underlying value.

    This function will panic if accessed from a different thread than the cell was created on.
    */
    #[inline]
    pub fn into_inner(self) -> T {
        assert_eq!(self.thread_id, crate::sys::thread::current().id());
        unsafe { self.into_unchecked_inner() }
    }

    /**
    Create a new cell with a new value, that will be runtime-checked against the same
    thread as the original cell.

    This is useful to implement simple clone/copy operations on the cell.

    # Safety
    * You must verify that the new value is safe to use on the same thread as the original cell.
    * Including that it can be dropped on that thread.
    */
    #[inline]
    pub unsafe fn preserving_cell_thread<U>(&self, new: U) -> SendCell<U> {
        SendCell {
            inner: Some(UnsafeSendCell::new_unchecked(new)),
            thread_id: self.thread_id,
        }
    }

    /**
    Copies the cell, creating a new cell that can be used on the same thread.

    # Safety
    This ought to be safe for types that implement Copy, since the copy constructor does not
    involve custom code.
*/
    pub fn copying(&self) -> Self where T: Copy {
        unsafe { self.preserving_cell_thread(*self.get_unchecked()) }
    }

}

impl<T> Drop for SendCell<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            assert_eq!(self.thread_id, crate::sys::thread::current().id(), "Drop SendCell from incorrect thread");
        }
    }
}

//implement boilerplate
impl<T: Debug> Debug for SendCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}


impl<T> AsRef<T> for SendCell<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> AsMut<T> for SendCell<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<T> Deref for SendCell<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for SendCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

//for eq, hash, etc, we generally rely on the underlying deref
impl<T: Default> Default for SendCell<T> {
    fn default() -> SendCell<T> {
        SendCell::new(Default::default())
    }
}
impl<T> From<T> for SendCell<T> {
    fn from(value: T) -> Self {
        SendCell::new(value)
    }
}



