//SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/**
A cell that can be sent across threads,
even if the underlying datatype is not Send.

Generally, all use of the cell is unsafe.

This cell type is appropriate for cases where the values is "not actually sent", but Rust thinks it is.

For cases where it might really be sent after all, consider using [crate::send_cell].
*/

pub struct UnsafeSendCell<T>(T);
unsafe impl<T> Send for UnsafeSendCell<T> {}

impl <T> UnsafeSendCell<T> {
    /**

    Creates a new cell.

    # Safety
    You must verify that e.g. Drop is safe for the underlying type.
*/
    #[inline]
    pub unsafe fn new_unchecked(value: T) -> Self {
        UnsafeSendCell(value)
    }

    /**
    Creates a new cell.

    # Safety

    This function is safe because it verifies that the underlying type does not implement Drop.
*/
    #[inline]
    pub fn new(value: T) -> Self {
        assert!(!std::mem::needs_drop::<T>(), "Cannot use safe constructor for types that implement Drop; use new_unchecked instead. ");
        UnsafeSendCell(value)
    }
    /**
    Gets the underlying value.

    # Safety
    Either
    a) the cell is never really sent, so the use of the cell is spurious here
    b) the cell is sent, but whatever we're doing is actually safe in some non-Rust way.
    */
    #[inline]
    pub unsafe fn get(&self) -> &T {
        &self.0
    }
    /**
    Gets the underlying value mutably.

    # Safety
    Either
    a) the cell is never really sent, so the use of the cell is spurious here
    b) the cell is sent, but whatever we're doing is actually safe in some non-Rust way.
    */
    #[inline]
    pub unsafe fn get_mut(&mut self) -> &mut T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        &mut self.0
    }

    /**
    Consumes the cell and returns the inner value.

    # Safety
    Either
    a) the cell is never really sent, so the use of the cell is spurious here
    b) the cell is sent, but whatever we're doing is actually safe in some non-Rust way.
    */
    #[inline]
    pub unsafe fn into_inner(self) -> T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        self.0
    }
}

impl<T: Future> UnsafeSendCell<T> {
    /**
    Converts the cell into a future that implements Send.

    # Safety
    The caller must verify that the future is not "actually" sent across threads.
    This method creates a wrapper that unsafely implements Send for the underlying future,
    which may violate Rust's memory safety guarantees if the future is truly sent
    between threads and accessed concurrently.
    */
    #[inline]
    pub unsafe fn into_future(self) -> UnsafeSendFuture<T> {
        UnsafeSendFuture(self.0)
    }
}

/**
A future wrapper that unsafely implements Send.

This wrapper allows futures that don't implement Send to be used in contexts
that require Send futures, but the caller must ensure the future is not
actually sent across threads.
*/
pub struct UnsafeSendFuture<T>(T);

unsafe impl<T> Send for UnsafeSendFuture<T> {}

impl<T: Future> Future for UnsafeSendFuture<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We're maintaining the pinning invariant by not moving the inner future
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        inner.poll(cx)
    }
}

/*
Design note about traits.

In general, &self functions cannot be implemented in Safe rust.  This rules out Debug, Clone, Copy,
PartialEq, Eq, PartialOrd, Ord, Hash,AsRef.   In general,
chain through calls to the unsafe fn `get()`.

For send, &mut and self cannot be implemented either – the idea is that the underlying type
is not necessarily portable across threads, implementing Send allows us to do so, and a value so
ported across threads is not necessarily valid on a new thread.

Default,From can be implemented as they work on owning types.


 */

impl<T: Default> Default for UnsafeSendCell<T> {
    fn default() -> Self {
        UnsafeSendCell(Default::default())
    }
}

impl<T> From<T> for UnsafeSendCell<T> {
    fn from(value: T) -> Self {
        UnsafeSendCell(value)
    }
}

impl<T> Debug for UnsafeSendCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //note that we can't safely access the underlying field here – it may have been sent.
        f.debug_tuple("SendCell")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::task::{Context, Poll};
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
    fn test_non_send_future_to_send_future() {
        // Create a non-Send future
        let non_send_future = NonSendFuture::new(42);
        
        // Verify it's not Send (this would fail to compile if uncommented)
        // assert_send(&non_send_future);
        
        // Wrap it in UnsafeSendCell
        let cell = unsafe { UnsafeSendCell::new_unchecked(non_send_future) };
        
        // Convert to a Send future
        let send_future = unsafe { cell.into_future() };
        
        // Verify the resulting future is Send
        assert_send(&send_future);
        
        // This demonstrates that we can now use this future in Send contexts
        // For example, we could spawn it on a thread pool (though we won't actually do that here)
    }

    #[test]
    fn test_future_functionality_preserved() {
        use std::task::{RawWaker, RawWakerVTable, Waker};
        
        // Create a non-Send future
        let non_send_future = NonSendFuture::new(42);
        
        // Wrap and convert
        let cell = unsafe { UnsafeSendCell::new_unchecked(non_send_future) };
        let mut send_future = unsafe { cell.into_future() };
        
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
}

