//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
Unsafe cells for sending non-Send types across thread boundaries without runtime checks.

This module provides [`UnsafeSendCell<T>`] and [`UnsafeSendFuture<T>`], which allow you to
wrap non-Send types and move them between threads without any runtime safety checks.
Unlike [`crate::send_cell`], this module requires `unsafe` blocks and manual verification
of thread safety.

# When to Use

This module is appropriate when:
- You have external guarantees that the data won't actually be accessed from multiple threads
- You're working with platform-specific APIs that guarantee callback execution on specific threads
- You need maximum performance and can manually verify thread safety
- You're prototyping and want to bypass the borrow checker temporarily

# Safety Requirements

When using these types, you must ensure:
- The wrapped value is never actually accessed concurrently from multiple threads
- Drop implementations are safe to run on any thread the value might end up on
- No thread-local state dependencies exist in the wrapped type
- External synchronization is provided when necessary

# Comparison with Safe Variants

| Type | Runtime Checks | Performance | Safety |
|------|---------------|-------------|--------|
| [`crate::SendCell`] | Yes | Slower | Memory safe |
| [`UnsafeSendCell`] | No | Faster | Requires manual verification |

# Examples

## Basic Usage

```rust
use send_cells::UnsafeSendCell;
use std::rc::Rc;

// Rc<T> is not Send, but we can wrap it unsafely
let data = Rc::new(42);
let cell = unsafe { UnsafeSendCell::new_unchecked(data) };

// The cell can now be moved between threads
fn requires_send<T: Send>(_: &T) {}
requires_send(&cell);

// SAFETY: We must ensure this is only accessed from the correct thread
let value = unsafe { cell.get() };
// println!("Value: {}", **value); // Only safe on the original thread
```

## With Types That Don't Implement Drop

```rust
use send_cells::UnsafeSendCell;

// For types without Drop, we can use the safe constructor
let cell = UnsafeSendCell::new(42i32);

// Safe to access since i32 has no thread-local state
let value = unsafe { cell.get() };
assert_eq!(*value, 42);
```

## Future Wrapping

```rust
use send_cells::UnsafeSendCell;
use std::rc::Rc;

// Create a non-Send future
async fn non_send_async() -> i32 {
    let _local = Rc::new(42); // Non-Send
    42
}

let future = non_send_async();
let cell = unsafe { UnsafeSendCell::new_unchecked(future) };
let send_future = unsafe { cell.into_future() };

// Now it can be used in Send contexts (but ONLY if you can guarantee
// it's not actually sent between threads or is safe to do so)
fn requires_send_future<F: std::future::Future + Send>(_: F) {}
requires_send_future(send_future);
```

# Platform-Specific Guarantees

This module is particularly useful when working with platform APIs that provide
thread guarantees that Rust can't verify:

```rust
use send_cells::UnsafeSendCell;
use std::rc::Rc;

// Example: Platform callback that's guaranteed to run on the main thread
fn setup_callback() {
    let data = Rc::new("main thread data");
    let cell = unsafe { UnsafeSendCell::new_unchecked(data) };

    // SAFETY: Platform guarantees this callback runs on the main thread
    some_platform_api(move || {
        let data = unsafe { cell.get() };
        println!("Callback data: {}", data);
    });
}
# fn some_platform_api<F: FnOnce() + Send + 'static>(_f: F) {} // Mock function
```

# Warning

Using this module incorrectly can lead to undefined behavior, data races,
and memory safety violations. Always prefer [`crate::SendCell`] unless you
have specific performance requirements and can manually verify safety.
*/

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A cell that can be sent across threads without runtime checks.
///
/// `UnsafeSendCell<T>` wraps a value of type `T` (which may not implement `Send`) and
/// provides an unsafe `Send` implementation. Unlike [`crate::SendCell`], this type
/// performs no runtime checks and requires manual verification of thread safety.
///
/// All access to the wrapped value requires `unsafe` blocks, making the safety
/// requirements explicit at the call site.
///
/// # Safety
///
/// When using `UnsafeSendCell<T>`, you must ensure:
/// - The wrapped value is never accessed concurrently from multiple threads
/// - If the value is moved between threads, it's safe to do so
/// - Drop implementations are safe to run on any thread
/// - External synchronization is provided when necessary
///
/// # Examples
///
/// ## With Non-Drop Types (Safe Constructor)
///
/// ```rust
/// use send_cells::UnsafeSendCell;
///
/// // i32 doesn't implement Drop, so this is safe
/// let cell = UnsafeSendCell::new(42);
/// let value = unsafe { cell.get() };
/// assert_eq!(*value, 42);
/// ```
///
/// ## With Drop Types (Unsafe Constructor)
///
/// ```rust
/// use send_cells::UnsafeSendCell;
/// use std::rc::Rc;
///
/// let data = Rc::new("hello");
///
/// // SAFETY: We guarantee this won't be accessed from multiple threads
/// let cell = unsafe { UnsafeSendCell::new_unchecked(data) };
///
/// // SAFETY: We're still on the original thread
/// let value = unsafe { cell.get() };
/// assert_eq!(**value, "hello");
/// ```
///
/// ## Thread Safety Verification
///
/// ```rust
/// use send_cells::UnsafeSendCell;
/// use std::rc::Rc;
///
/// fn assert_send<T: Send>(_: T) {}
///
/// let data = Rc::new(42);
/// let cell = unsafe { UnsafeSendCell::new_unchecked(data) };
///
/// // The cell implements Send even though Rc<T> doesn't
/// assert_send(cell);
/// ```
///
/// # When to Use
///
/// This type is appropriate when:
/// - You have platform guarantees about thread usage
/// - Maximum performance is required and safety can be manually verified
/// - Working with callback-based APIs with thread guarantees
/// - Prototyping concurrent code
///
/// For safer alternatives with runtime checks, see [`crate::SendCell`].
pub struct UnsafeSendCell<T>(T);

// SAFETY: UnsafeSendCell implements Send for any T, regardless of whether T implements Send.
// This is unsafe and requires the user to manually verify that the value won't be accessed
// concurrently from multiple threads.
unsafe impl<T> Send for UnsafeSendCell<T> {}

impl<T> UnsafeSendCell<T> {
    /// Creates a new cell without verifying thread safety.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - The value `T` can be safely moved between threads
    /// - If `T` implements `Drop`, it's safe to drop on any thread
    /// - The value won't be accessed concurrently from multiple threads
    /// - Any thread-local state dependencies are properly handled
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    /// use std::rc::Rc;
    ///
    /// let data = Rc::new(42);
    ///
    /// // SAFETY: We guarantee this won't be shared between threads
    /// let cell = unsafe { UnsafeSendCell::new_unchecked(data) };
    /// ```
    #[inline]
    pub unsafe fn new_unchecked(value: T) -> Self {
        UnsafeSendCell(value)
    }

    /// Creates a new cell for types that don't implement Drop.
    ///
    /// This constructor is safe because it statically verifies that the type `T`
    /// does not implement `Drop`. Types without custom drop implementations are
    /// generally safe to move between threads (assuming no other thread-local
    /// dependencies).
    ///
    /// # Panics
    ///
    /// Panics if `T` implements `Drop`. Use [`Self::new_unchecked`] for types
    /// that implement `Drop`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    ///
    /// // i32 doesn't implement Drop, so this is safe
    /// let cell = UnsafeSendCell::new(42);
    /// let value = unsafe { cell.get() };
    /// assert_eq!(*value, 42);
    /// ```
    ///
    /// ```should_panic
    /// use send_cells::UnsafeSendCell;
    /// use std::rc::Rc;
    ///
    /// // This will panic because Rc<T> implements Drop
    /// let data = Rc::new(42);
    /// let cell = UnsafeSendCell::new(data); // Panics!
    /// ```
    #[inline]
    pub fn new(value: T) -> Self {
        assert!(
            !std::mem::needs_drop::<T>(),
            "Cannot use safe constructor for types that implement Drop; use new_unchecked instead. "
        );
        UnsafeSendCell(value)
    }
    /// Gets a reference to the underlying value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is concurrently accessing the value
    /// - The value is safe to access from the current thread
    /// - All thread-safety invariants are maintained
    ///
    /// This method is unsafe because it bypasses Rust's normal Send/Sync checking.
    /// The safety requirements depend on your specific use case:
    /// - If the cell was never actually sent between threads, this is safe
    /// - If the cell was sent between threads, you must have external guarantees
    ///   about thread safety
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    ///
    /// let cell = UnsafeSendCell::new(42);
    ///
    /// // SAFETY: We're on the same thread and i32 is safe to access
    /// let value = unsafe { cell.get() };
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    pub unsafe fn get(&self) -> &T {
        &self.0
    }
    /// Gets a mutable reference to the underlying value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is concurrently accessing the value
    /// - The value is safe to access mutably from the current thread
    /// - No other references (mutable or immutable) to the value exist
    /// - All thread-safety invariants are maintained
    ///
    /// This method is unsafe because it bypasses Rust's normal Send/Sync checking
    /// and because mutable access requires exclusive access guarantees.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    ///
    /// let mut cell = UnsafeSendCell::new(42);
    ///
    /// // SAFETY: We have exclusive access and i32 is safe to mutate
    /// unsafe {
    ///     *cell.get_mut() = 100;
    /// }
    ///
    /// let value = unsafe { cell.get() };
    /// assert_eq!(*value, 100);
    /// ```
    #[inline]
    pub unsafe fn get_mut(&mut self) -> &mut T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        &mut self.0
    }

    /// Consumes the cell and returns the wrapped value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - It's safe to take ownership of the value on the current thread
    /// - The value can be safely dropped on the current thread
    /// - No other references to the value exist
    ///
    /// This method is unsafe because it bypasses Rust's normal Send checking
    /// when taking ownership of the value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    ///
    /// let cell = UnsafeSendCell::new(42);
    ///
    /// // SAFETY: i32 is safe to take ownership of on any thread
    /// let value = unsafe { cell.into_inner() };
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    pub unsafe fn into_inner(self) -> T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        self.0
    }
}

impl<T: Future> UnsafeSendCell<T> {
    /// Converts the cell into a future that implements Send.
    ///
    /// This method consumes the `UnsafeSendCell` and returns an [`UnsafeSendFuture`]
    /// that implements `Send`. The returned future can be moved between threads
    /// but requires the same safety guarantees as the original cell.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - The future won't be polled concurrently from multiple threads
    /// - If the future is moved between threads, it's safe to do so
    /// - The future's state and any captured variables are thread-safe
    /// - Drop implementations are safe to run on any thread
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::UnsafeSendCell;
    /// use std::rc::Rc;
    ///
    /// async fn non_send_future() -> i32 {
    ///     let _local = Rc::new(42); // Non-Send
    ///     42
    /// }
    ///
    /// let future = non_send_future();
    ///
    /// // SAFETY: We guarantee this future won't be sent between threads
    /// let cell = unsafe { UnsafeSendCell::new_unchecked(future) };
    /// let send_future = unsafe { cell.into_future() };
    ///
    /// // Now it can be used in Send contexts
    /// fn requires_send<T: Send>(_: T) {}
    /// requires_send(send_future);
    /// ```
    #[inline]
    pub unsafe fn into_future(self) -> UnsafeSendFuture<T> {
        UnsafeSendFuture(self.0)
    }
}

/// A future wrapper that unsafely implements Send.
///
/// `UnsafeSendFuture<T>` wraps a future of type `T` and provides an unsafe `Send`
/// implementation. This allows non-Send futures to be used in contexts that require
/// Send futures, but requires manual verification of thread safety.
///
/// Unlike [`crate::SendFuture`], this type performs no runtime checks and all
/// usage requires careful safety analysis.
///
/// # Safety
///
/// When using `UnsafeSendFuture<T>`, you must ensure:
/// - The future is never polled concurrently from multiple threads
/// - If moved between threads, the future's state is safe to access
/// - Any captured variables in the future are thread-safe
/// - Drop implementations are safe to run on any thread
///
/// # Examples
///
/// ```rust
/// use send_cells::{UnsafeSendCell, UnsafeSendFuture};
/// use std::rc::Rc;
/// use std::future::Future;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
///
/// // A future that captures non-Send data
/// struct NonSendFuture {
///     data: Rc<i32>,
/// }
///
/// impl Future for NonSendFuture {
///     type Output = i32;
///     
///     fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
///         Poll::Ready(*self.data)
///     }
/// }
///
/// let future = NonSendFuture { data: Rc::new(42) };
///
/// // SAFETY: We guarantee this won't be sent between threads
/// let cell = unsafe { UnsafeSendCell::new_unchecked(future) };
/// let send_future = unsafe { cell.into_future() };
///
/// // Verify it implements Send
/// fn assert_send<T: Send>(_: T) {}
/// assert_send(send_future);
/// ```
///
/// # Performance
///
/// This wrapper has zero runtime overhead compared to the underlying future,
/// making it suitable for performance-critical applications where safety
/// can be manually verified.
#[derive(Debug)]
pub struct UnsafeSendFuture<T>(T);

// SAFETY: UnsafeSendFuture implements Send for any T, regardless of whether T implements Send.
// This is unsafe and requires the user to manually verify that the future won't be accessed
// concurrently from multiple threads.
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
        // Note: We can't safely access the underlying field here because it may have been sent
        // to a different thread where accessing it would be unsafe.
        f.debug_tuple("UnsafeSendCell")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

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

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
