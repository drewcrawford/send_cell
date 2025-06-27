//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
A runtime-checked sending cell.

This verifies that all use of the resulting value occurs on the same thread.
*/

use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::thread::ThreadId;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
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

impl<T: Future> SendCell<T> {
    /**
    Converts the cell into a future that implements Send with runtime thread checking.
    
    Unlike UnsafeSendCell's into_future(), this method creates a future that will
    panic if polled from a different thread than the one where the SendCell was created.
    This provides safe cross-thread future usage by enforcing thread safety at runtime.
    
    # Panics
    
    The returned future will panic if polled from a different thread than the one
    where this SendCell was created.
    */
    pub fn into_future(mut self) -> SendFuture<T> {
        SendFuture {
            inner: self.inner.take().expect("inner value missing"),
            thread_id: self.thread_id,
        }
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

/**
A future wrapper that implements Send with runtime thread checking.

This wrapper allows futures to be used in contexts that require Send futures,
while ensuring thread safety by checking that poll() is only called from the
correct thread. Unlike UnsafeSendFuture, this provides safe cross-thread usage
by panicking if accessed from the wrong thread.
*/
#[derive(Debug)]
pub struct SendFuture<T> {
    inner: UnsafeSendCell<T>,
    thread_id: ThreadId,
}

unsafe impl<T> Send for SendFuture<T> {}

impl<T: Future> Future for SendFuture<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Runtime thread check - panic if called from wrong thread
        assert_eq!(
            self.thread_id, 
            crate::sys::thread::current().id(), 
            "SendFuture polled from incorrect thread"
        );
        
        // SAFETY: After the thread check, we can safely access the inner future
        // using the same technique as UnsafeSendFuture
        let inner = unsafe { 
            let self_mut = self.get_unchecked_mut();
            Pin::new_unchecked(self_mut.inner.get_mut())
        };
        inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    use std::pin::Pin;

    // A future that is NOT Send because it contains Rc<T>
    struct NonSendFuture {
        _data: Rc<i32>,
        ready: bool,
    }

    impl NonSendFuture {
        fn new(value: i32) -> Self {
            Self {
                _data: Rc::new(value),
                ready: false,
            }
        }
    }

    impl Future for NonSendFuture {
        type Output = i32;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready {
                Poll::Ready(42)
            } else {
                self.ready = true;
                Poll::Pending
            }
        }
    }

    // Helper function to verify a type implements Send
    fn assert_send<T: Send>(_: &T) {}

    #[test]
    fn test_send_cell_into_future_is_send() {
        // Create a non-Send future
        let non_send_future = NonSendFuture::new(42);
        
        // Wrap it in SendCell
        let cell = SendCell::new(non_send_future);
        
        // Convert to a Send future
        let send_future = cell.into_future();
        
        // Verify the resulting future is Send
        assert_send(&send_future);
    }

    #[test]
    fn test_send_future_functionality() {
        // Create a no-op waker for testing
        static VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        let raw_waker = RawWaker::new(std::ptr::null(), &VTABLE);
        let waker = unsafe { Waker::from_raw(raw_waker) };
        let mut context = Context::from_waker(&waker);
        
        // Create a non-Send future wrapped in SendCell
        let non_send_future = NonSendFuture::new(42);
        let cell = SendCell::new(non_send_future);
        let mut send_future = cell.into_future();
        
        // Test that the future still works correctly
        let pinned = Pin::new(&mut send_future);
        match pinned.poll(&mut context) {
            Poll::Pending => {
                // First poll should return Pending
                let pinned = Pin::new(&mut send_future);
                match pinned.poll(&mut context) {
                    Poll::Ready(value) => assert_eq!(value, 42),
                    Poll::Pending => panic!("Expected Ready on second poll"),
                }
            }
            Poll::Ready(value) => assert_eq!(value, 42),
        }
    }

    #[test]
    fn test_send_future_cross_thread_panic() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        // Create future on main thread
        let non_send_future = NonSendFuture::new(42);
        let cell = SendCell::new(non_send_future);
        let send_future = cell.into_future();
        
        // Share the future with another thread
        let future_mutex = Arc::new(Mutex::new(send_future));
        let future_clone = Arc::clone(&future_mutex);
        
        // Try to poll from a different thread - this should panic
        let handle = thread::spawn(move || {
            // Create a no-op waker inside the thread
            static VTABLE: RawWakerVTable = RawWakerVTable::new(
                |_| RawWaker::new(std::ptr::null(), &VTABLE),
                |_| {},
                |_| {},
                |_| {},
            );
            let raw_waker = RawWaker::new(std::ptr::null(), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw_waker) };
            let mut context = Context::from_waker(&waker);
            
            let mut future_guard = future_clone.lock().unwrap();
            let pinned = Pin::new(&mut *future_guard);
            let _ = pinned.poll(&mut context);
        });
        
        // Verify that the thread panicked
        let result = handle.join();
        assert!(result.is_err(), "Expected thread to panic when polling SendFuture from incorrect thread");
    }
}



