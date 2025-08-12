// SPDX-License-Identifier: MIT OR Apache-2.0
/*!
Unsafe cells for sharing non-Sync types across thread boundaries without synchronization.

This module provides [`UnsafeSyncCell<T>`], which allows you to wrap non-Sync types
and share them between threads without any synchronization or runtime safety checks.
Unlike [`crate::sync_cell`], this module requires `unsafe` blocks and manual verification
of thread safety.

# When to Use

This module is appropriate when:
- You have external guarantees that data won't be accessed concurrently
- You're working with platform-specific APIs that provide implicit synchronization
- You need maximum performance and can manually verify thread safety
- You're prototyping and want to bypass Rust's Sync requirements temporarily

# Safety Requirements

When using these types, you must ensure:
- The wrapped value is never accessed concurrently from multiple threads
- External synchronization is provided when necessary
- No data races occur during access
- Drop implementations are safe to run on any thread

# Comparison with Safe Variants

| Type | Synchronization | Performance | Safety |
|------|----------------|-------------|--------|
| [`crate::SyncCell`] | Mutex | Slower | Memory safe |
| [`UnsafeSyncCell`] | None | Faster | Requires manual verification |

# Examples

## Basic Usage

```rust
use send_cells::unsafe_sync_cell::UnsafeSyncCell;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;

// RefCell<T> is not Sync, but we can wrap it unsafely
let data = RefCell::new(42);
let cell = UnsafeSyncCell::new(data);
let shared_cell = Arc::new(cell);

// SAFETY: We must ensure no concurrent access occurs
let cell_clone = Arc::clone(&shared_cell);
let handle = thread::spawn(move || {
    // SAFETY: External synchronization ensures no concurrent access
    let value = unsafe { cell_clone.get() };
    println!("Value: {}", *value.borrow());
});

handle.join().unwrap();
```

## With External Synchronization

```rust
use send_cells::unsafe_sync_cell::UnsafeSyncCell;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::thread;

let data = RefCell::new(vec![1, 2, 3]);
let cell = Arc::new(UnsafeSyncCell::new(data));
let mutex = Arc::new(Mutex::new(()));

let cell_clone = Arc::clone(&cell);
let mutex_clone = Arc::clone(&mutex);

thread::spawn(move || {
    let _guard = mutex_clone.lock().unwrap();
    // SAFETY: Mutex ensures exclusive access
    unsafe {
        let vec = cell_clone.get();
        println!("Length: {}", vec.borrow().len());
    }
}).join().unwrap();

// Main thread can also access after the other thread is done
let _guard = mutex.lock().unwrap();
unsafe {
    let vec = cell.get();
    assert_eq!(vec.borrow().len(), 3);
}
```

# Platform-Specific Usage

This module is particularly useful with platform APIs that provide implicit guarantees:

```rust
use send_cells::unsafe_sync_cell::UnsafeSyncCell;
use std::cell::RefCell;
use std::sync::Arc;

// Example: Single-threaded event loop with thread-safe callbacks
fn setup_event_handler() {
    let data = RefCell::new("event data");
    let cell = Arc::new(UnsafeSyncCell::new(data));

    // SAFETY: Platform guarantees all callbacks run on the main thread
    register_event_callback(move || {
        let data = unsafe { cell.get() };
        println!("Event: {}", *data.borrow());
    });
}
# fn register_event_callback<F: Fn() + Send + Sync + 'static>(_f: F) {} // Mock function
```

# Warning

Using this module incorrectly can lead to undefined behavior, data races,
and memory safety violations. Always prefer [`crate::SyncCell`] unless you
have specific performance requirements and can manually verify safety.
*/

use std::cell::UnsafeCell;
use std::fmt::Debug;

/// A cell that can be shared between threads without synchronization.
///
/// `UnsafeSyncCell<T>` wraps a value of type `T` (which may not implement `Sync`) and provides
/// an unsafe `Sync` implementation. Unlike [`crate::SyncCell`], this type performs no
/// synchronization and requires manual verification of thread safety.
///
/// All access to the wrapped value (except through `get_mut()`) requires `unsafe` blocks,
/// making the safety requirements explicit at the call site.
///
/// # Safety
///
/// When using `UnsafeSyncCell<T>`, you must ensure:
/// - The wrapped value is never accessed concurrently from multiple threads
/// - External synchronization is provided when necessary
/// - No data races occur during access
/// - Drop implementations are safe to run on any thread
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
/// use std::rc::Rc;
///
/// // Rc<i32> is not Sync, but UnsafeSyncCell<Rc<i32>> is
/// let data = Rc::new(42);
/// let cell = UnsafeSyncCell::new(data);
///
/// // SAFETY: We're not accessing concurrently
/// let value = unsafe { cell.get() };
/// assert_eq!(**value, 42);
/// ```
///
/// ## With External Synchronization
///
/// ```rust
/// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
/// use std::cell::RefCell;
/// use std::sync::{Arc, Mutex};
/// use std::thread;
///
/// let data = RefCell::new(vec![1, 2, 3]);
/// let cell = Arc::new(UnsafeSyncCell::new(data));
/// let mutex = Arc::new(Mutex::new(()));
///
/// let cell_clone = Arc::clone(&cell);
/// let mutex_clone = Arc::clone(&mutex);
///
/// thread::spawn(move || {
///     let _guard = mutex_clone.lock().unwrap();
///     // SAFETY: Mutex ensures exclusive access
///     unsafe {
///         let vec = cell_clone.get();
///         println!("Length: {}", vec.borrow().len());
///     }
/// }).join().unwrap();
/// ```
///
/// ## Mutable Access
///
/// ```rust
/// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
/// use std::collections::HashMap;
///
/// let map = HashMap::new();
/// let mut cell = UnsafeSyncCell::new(map);
///
/// // Safe mutable access (requires &mut self)
/// cell.get_mut().insert("key", "value");
///
/// // SAFETY: No concurrent access
/// unsafe {
///     let map = cell.get();
///     assert_eq!(map.get("key"), Some(&"value"));
/// }
/// ```
///
/// # When to Use
///
/// This type is appropriate when:
/// - You have platform guarantees about thread usage
/// - Maximum performance is required and safety can be manually verified
/// - Working with single-threaded event loops or callback systems
/// - Prototyping concurrent code
///
/// For safer alternatives with automatic synchronization, see [`crate::SyncCell`].
pub struct UnsafeSyncCell<T>(UnsafeCell<T>);

// SAFETY: UnsafeSyncCell implements Sync for any T, regardless of whether T implements Sync.
// This is unsafe and requires the user to manually verify that concurrent access won't occur
// or that external synchronization is provided.
unsafe impl<T> Sync for UnsafeSyncCell<T> {}

impl<T> UnsafeSyncCell<T> {
    /// Creates a new `UnsafeSyncCell` wrapping the given value.
    ///
    /// The value will be wrapped in an [`std::cell::UnsafeCell`] and can be shared
    /// between threads, but requires manual verification of thread safety for all access.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
    /// use std::rc::Rc;
    ///
    /// let data = Rc::new("Hello, world!");
    /// let cell = UnsafeSyncCell::new(data);
    ///
    /// // SAFETY: No concurrent access
    /// let value = unsafe { cell.get() };
    /// println!("{}", value);
    /// ```
    #[inline]
    pub fn new(value: T) -> Self {
        UnsafeSyncCell(UnsafeCell::new(value))
    }
    /// Gets a reference to the underlying value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is concurrently accessing the value mutably
    /// - The value is safe to access from the current thread
    /// - If multiple threads access concurrently, they only perform read operations
    /// - External synchronization is provided if needed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
    ///
    /// let cell = UnsafeSyncCell::new(42);
    ///
    /// // SAFETY: Single-threaded access
    /// let value = unsafe { cell.get() };
    /// assert_eq!(*value, 42);
    /// ```
    pub unsafe fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
    /// Gets a mutable reference to the underlying value.
    ///
    /// This is safe because it requires a mutable reference to the cell itself,
    /// which the borrow checker guarantees is exclusive. For cases where you need
    /// mutable access from a shared reference, use [`Self::get_mut_unchecked`] with
    /// appropriate safety guarantees.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// let mut cell = UnsafeSyncCell::new(map);
    ///
    /// // Safe because we have &mut access
    /// cell.get_mut().insert("key", "value");
    ///
    /// // SAFETY: No concurrent access
    /// unsafe {
    ///     let map = cell.get();
    ///     assert_eq!(map.get("key"), Some(&"value"));
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        self.0.get_mut()
    }

    /// Gets a mutable reference to the underlying value without requiring `&mut self`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is concurrently accessing the value
    /// - No other references (mutable or immutable) to the value exist
    /// - The access is properly synchronized through external means
    /// - All thread-safety invariants are maintained
    ///
    /// This method is particularly dangerous because it allows mutable access
    /// from a shared reference, bypassing Rust's aliasing rules.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::unsafe_sync_cell::UnsafeSyncCell;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let cell = Arc::new(UnsafeSyncCell::new(42));
    /// let mutex = Arc::new(Mutex::new(()));
    ///
    /// {
    ///     let _guard = mutex.lock().unwrap();
    ///     // SAFETY: Mutex ensures exclusive access
    ///     unsafe {
    ///         *cell.get_mut_unchecked() = 100;
    ///     }
    /// }
    ///
    /// // SAFETY: Mutex released, no concurrent access
    /// unsafe {
    ///     assert_eq!(*cell.get(), 100);
    /// }
    /// ```
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut_unchecked(&self) -> &mut T {
        //This is unsafe because it allows you to mutate the value without a mutable reference to the cell.
        //You must guarantee that you are the only one mutating the value.
        unsafe { &mut *self.0.get() }
    }

    /**
    Consumes the SyncCell and returns the inner value.
    */
    pub fn into_inner(self) -> T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        self.0.into_inner()
    }
}

/*
Design note about traits.

In general, &self functions cannot be implemented in Safe rust.  This rules out Debug, Clone, Copy,
PartialEq, Eq, PartialOrd, Ord, Hash,AsRef.   In general,
chain through calls to the unsafe fn `get()`.

Default,From can be implemented as they work on owning types.

AsMut can be implemented, since we have an exclusive reference.

DerefMut can't be implemented due to lack of deref type.

 */

impl<T> Debug for UnsafeSyncCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //we can't use the value here since we can't guarantee it's safe to do so.
        //but we can use the type name
        f.debug_tuple("SyncCell")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

impl<T: Default> Default for UnsafeSyncCell<T> {
    fn default() -> Self {
        UnsafeSyncCell(T::default().into())
    }
}

impl<T> From<T> for UnsafeSyncCell<T> {
    fn from(value: T) -> Self {
        UnsafeSyncCell(value.into())
    }
}

impl<T> AsMut<T> for UnsafeSyncCell<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
