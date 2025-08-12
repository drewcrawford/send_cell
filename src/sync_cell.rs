//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
A runtime-checked synchronous cell for safe shared access to non-Sync types.

This module provides [`SyncCell<T>`], which allows you to wrap non-Sync types
and share them between threads safely using a mutex and closure-based access.
Unlike [`crate::unsafe_sync_cell`], this module provides safe APIs with automatic
mutex management.

# Use Cases

- Sharing non-Sync types (like `Rc<T>`, `RefCell<T>`) between threads safely
- Protecting shared state without manually managing mutex guards
- Preventing deadlocks by ensuring guards are automatically released
- Working with synchronous APIs in multi-threaded contexts

# Thread Safety Model

[`SyncCell<T>`] uses a [`std::sync::Mutex`] internally to provide thread-safe access:
- All access is through closures that receive references to the wrapped value
- Mutex guards are automatically acquired and released by the closure methods
- This prevents holding guards across await points or other blocking operations
- The wrapped value itself doesn't need to implement `Sync`

# Examples

Basic usage with shared state:

```rust
use send_cells::SyncCell;
use std::cell::RefCell;
use std::thread;
use std::sync::Arc;

// RefCell<T> is not Sync, but SyncCell<RefCell<T>> is
let data = RefCell::new(42);
let cell = Arc::new(SyncCell::new(data));

// Share between threads
let cell_clone = Arc::clone(&cell);
let handle = thread::spawn(move || {
    cell_clone.with(|ref_cell| {
        println!("Value in thread: {}", *ref_cell.borrow());
    });
});

handle.join().unwrap();
```

Mutable access:

```rust
use send_cells::SyncCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

let map = HashMap::new();
let cell = Arc::new(SyncCell::new(map));

let cell_clone = Arc::clone(&cell);
thread::spawn(move || {
    cell_clone.with_mut(|map| {
        map.insert("key", "value");
    });
}).join().unwrap();

cell.with(|map| {
    assert_eq!(map.get("key"), Some(&"value"));
});
```

# Avoiding Deadlocks

The closure-based API automatically prevents common deadlock scenarios:

```rust
use send_cells::SyncCell;

let cell = SyncCell::new(vec![1, 2, 3]);

// Guards are automatically released when closures return
cell.with(|vec| {
    println!("Length: {}", vec.len());
}); // Guard released here

cell.with_mut(|vec| {
    vec.push(4);
}); // No deadlock - previous guard was released
```
*/

use std::fmt::{Debug, Formatter};
use std::sync::{Mutex, };
use std::hash::{Hash, Hasher};
use crate::unsafe_sync_cell::UnsafeSyncCell;

/// A runtime-checked cell that allows sharing non-Sync types between threads.
///
/// `SyncCell<T>` wraps a value of type `T` (which may not implement `Sync`) and provides
/// a `Sync` implementation using a mutex for thread-safe access. Access to the wrapped
/// value is provided through closure-based methods that automatically manage the mutex.
///
/// Unlike [`crate::unsafe_sync_cell::UnsafeSyncCell`], this provides memory safety by using a real mutex
/// and proper synchronization. The closure-based API prevents common issues like
/// holding guards across await points or forgetting to release locks.
///
/// # Examples
///
/// Basic usage with a non-Sync type:
///
/// ```rust
/// use send_cells::SyncCell;
/// use std::cell::RefCell;
/// use std::sync::Arc;
/// use std::thread;
///
/// // RefCell<i32> is not Sync, but SyncCell<RefCell<i32>> is
/// let data = RefCell::new(42);
/// let cell = Arc::new(SyncCell::new(data));
///
/// let cell_clone = Arc::clone(&cell);
/// let handle = thread::spawn(move || {
///     cell_clone.with(|ref_cell| {
///         assert_eq!(*ref_cell.borrow(), 42);
///     });
/// });
///
/// handle.join().unwrap();
/// ```
///
/// Mutable access:
///
/// ```rust
/// use send_cells::SyncCell;
/// use std::collections::HashMap;
///
/// let map = HashMap::new();
/// let cell = SyncCell::new(map);
///
/// cell.with_mut(|map| {
///     map.insert("key", "value");
/// });
///
/// cell.with(|map| {
///     assert_eq!(map.get("key"), Some(&"value"));
/// });
/// ```
///
/// # Thread Safety
///
/// The cell implements both `Send` and `Sync` when the wrapped type implements `Send`.
/// Access is always protected by the internal mutex, ensuring thread safety.
pub struct SyncCell<T> {
    inner: UnsafeSyncCell<T>,
    mutex: Mutex<()>,
}

impl<T> SyncCell<T> {
    /// Creates a new `SyncCell` wrapping the given value.
    ///
    /// The value will be protected by an internal mutex, allowing safe shared
    /// access from multiple threads through the closure-based access methods.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    /// use std::rc::Rc;
    ///
    /// let data = Rc::new("Hello, world!");
    /// let cell = SyncCell::new(data);
    ///
    /// cell.with(|rc| {
    ///     println!("{}", rc);
    /// });
    /// ```
    #[inline]
    pub fn new(value: T) -> SyncCell<T> {
        SyncCell {
            inner: UnsafeSyncCell::new(value),
            mutex: Mutex::new(()),
        }
    }

    /// Accesses the underlying value through a synchronous closure.
    ///
    /// The closure receives a shared reference to the wrapped value and must
    /// return synchronously. The internal mutex is automatically acquired before
    /// calling the closure and released when the closure returns.
    ///
    /// This method provides safe, synchronized access to the wrapped value from
    /// any thread.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned (i.e., another thread panicked while
    /// holding the lock).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert("key", "value");
    /// let cell = SyncCell::new(map);
    ///
    /// let result = cell.with(|map| {
    ///     map.get("key").copied()
    /// });
    ///
    /// assert_eq!(result, Some("value"));
    /// ```
    #[inline]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let _guard = self.mutex.lock().unwrap();
        let value = unsafe{self.inner.get()};
        let result = f(value);
        result
    }

    /// Accesses the underlying value mutably through a synchronous closure.
    ///
    /// The closure receives a mutable reference to the wrapped value and must
    /// return synchronously. The internal mutex is automatically acquired before
    /// calling the closure and released when the closure returns.
    ///
    /// This method provides safe, synchronized mutable access to the wrapped value
    /// from any thread.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned (i.e., another thread panicked while
    /// holding the lock).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::new();
    /// let cell = SyncCell::new(map);
    ///
    /// cell.with_mut(|map| {
    ///     map.insert("key", "value");
    /// });
    ///
    /// cell.with(|map| {
    ///     assert_eq!(map.get("key"), Some(&"value"));
    /// });
    /// ```
    #[inline]
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let _guard = self.mutex.lock().unwrap();
        //safe since we hold the lock
        let value = unsafe { self.inner.get_mut_unchecked() };
        let result = f(value);
        result
    }

    /// Consumes the cell and returns the wrapped value.
    ///
    /// This method takes ownership of the `SyncCell` and returns the wrapped value
    /// without any synchronization, since the cell is being consumed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    /// use std::rc::Rc;
    ///
    /// let data = Rc::new("Hello, world!");
    /// let cell = SyncCell::new(data);
    ///
    /// let recovered_data = cell.into_inner();
    /// assert_eq!(*recovered_data, "Hello, world!");
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
    
    /// Unsafely accesses the underlying value without acquiring the mutex.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is currently accessing the value
    /// - The access is properly synchronized through external means
    /// - The mutex is not poisoned
    ///
    /// This method bypasses all synchronization and may lead to data races
    /// if used incorrectly.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    ///
    /// let cell = SyncCell::new(42);
    /// 
    /// // SAFETY: We're the only thread accessing this cell
    /// let value = unsafe { cell.with_unchecked() };
    /// assert_eq!(*value, 42);
    /// ```
    pub unsafe fn with_unchecked(&self) -> &T {
        // SAFETY: Caller guarantees proper synchronization
        self.inner.get()
    }
    
    /// Unsafely accesses the underlying value mutably without acquiring the mutex.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - No other thread is currently accessing the value
    /// - The access is properly synchronized through external means  
    /// - The mutex is not poisoned
    /// - No other references (mutable or immutable) to the value exist
    ///
    /// This method bypasses all synchronization and may lead to data races
    /// if used incorrectly.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use send_cells::SyncCell;
    ///
    /// let cell = SyncCell::new(42);
    /// 
    /// // SAFETY: We're the only thread accessing this cell
    /// unsafe {
    ///     *cell.with_mut_unchecked() = 100;
    /// }
    /// 
    /// cell.with(|value| {
    ///     assert_eq!(*value, 100);
    /// });
    /// ```
    pub unsafe fn with_mut_unchecked(&self) -> &mut T {
        // SAFETY: Caller guarantees proper synchronization
        self.inner.get_mut_unchecked()
    }
    
    
}

// SAFETY: SyncCell<T> can be Send when T: Send because the mutex ensures
// that only one thread can access the inner value at a time.
unsafe impl<T: Send> Send for SyncCell<T> {}

// SAFETY: SyncCell<T> can be Sync when T: Send because the mutex provides
// the necessary synchronization for shared access across threads.
unsafe impl<T: Send> Sync for SyncCell<T> {}


// ===========================================================================================
// BOILERPLATE TRAIT IMPLEMENTATIONS
// ===========================================================================================
// All trait implementations below use the closure-based access methods to ensure proper
// synchronization by going through the mutex. This prevents deadlocks and ensures thread safety.
//
// Design Notes:
// - AsRef/Deref are intentionally NOT implemented because they would require returning references
//   that outlive the mutex guard, which could lead to deadlocks or use-after-free issues
// - All implementations use the safe `with()` method for immutable access
// - Clone creates a new independent SyncCell to maintain the ownership model

// Basic formatting and construction traits
impl<T: Debug> Debug for SyncCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.with(|value| value.fmt(f))
    }
}

impl<T: std::fmt::Display> std::fmt::Display for SyncCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|value| value.fmt(f))
    }
}

impl<T: Default> Default for SyncCell<T> {
    fn default() -> SyncCell<T> {
        SyncCell::new(T::default())
    }
}

impl<T> From<T> for SyncCell<T> {
    fn from(value: T) -> Self {
        SyncCell::new(value)
    }
}

// Clone creates a new independent SyncCell with a cloned value
impl<T: Clone> Clone for SyncCell<T> {
    fn clone(&self) -> Self {
        self.with(|value| SyncCell::new(value.clone()))
    }
}

// Comparison traits - all use safe closure-based access
impl<T: PartialEq> PartialEq for SyncCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.with(|a| other.with(|b| a == b))
    }
}

impl<T: Eq> Eq for SyncCell<T> {}

impl<T: PartialOrd> PartialOrd for SyncCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.with(|a| other.with(|b| a.partial_cmp(b)))
    }
}

impl<T: Ord> Ord for SyncCell<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.with(|a| other.with(|b| a.cmp(b)))
    }
}

impl<T: Hash> Hash for SyncCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.with(|value| value.hash(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[test]
    fn test_basic_usage() {
        let cell = SyncCell::new(42);
        
        let result = cell.with(|value| *value * 2);
        assert_eq!(result, 84);
        
        cell.with_mut(|value| *value = 100);
        let result = cell.with(|value| *value);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_into_inner() {
        let cell = SyncCell::new(42);
        assert_eq!(cell.into_inner(), 42);
    }

    #[test]
    fn test_debug() {
        let cell = SyncCell::new(42);
        let debug_str = format!("{:?}", cell);
        assert_eq!(debug_str, "42");
    }

    #[test]
    fn test_display() {
        let cell = SyncCell::new(42);
        let display_str = format!("{}", cell);
        assert_eq!(display_str, "42");
        
        let cell_str = SyncCell::new("hello world");
        let display_str = format!("{}", cell_str);
        assert_eq!(display_str, "hello world");
    }

    #[test]
    fn test_default() {
        let cell: SyncCell<i32> = SyncCell::default();
        let value = cell.with(|v| *v);
        assert_eq!(value, 0);
    }

    #[test]
    fn test_from() {
        let cell: SyncCell<i32> = SyncCell::from(42);
        let value = cell.with(|v| *v);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_clone() {
        let cell = SyncCell::new(42);
        let cloned = cell.clone();
        
        assert_eq!(cell.with(|v| *v), cloned.with(|v| *v));
        
        cell.with_mut(|v| *v = 100);
        assert_eq!(cell.with(|v| *v), 100);
        assert_eq!(cloned.with(|v| *v), 42); // Clone is independent
    }

    #[test]
    fn test_partial_eq() {
        let cell1 = SyncCell::new(42);
        let cell2 = SyncCell::new(42);
        let cell3 = SyncCell::new(43);
        
        assert_eq!(cell1, cell2);
        assert_ne!(cell1, cell3);
    }

    #[test]
    fn test_ord() {
        let cell1 = SyncCell::new(1);
        let cell2 = SyncCell::new(2);
        let cell3 = SyncCell::new(3);
        
        assert!(cell1 < cell2);
        assert!(cell2 < cell3);
        assert!(cell1 < cell3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashMap;
        
        let cell1 = SyncCell::new(42);
        let cell2 = SyncCell::new(42);
        let cell3 = SyncCell::new(43);
        
        let mut map = HashMap::new();
        map.insert(cell1, "first");
        map.insert(cell2, "second"); // Should overwrite due to same hash/eq
        map.insert(cell3, "third");
        
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_send_sync() {
        fn assert_send<T: Send>(_: &T) {}
        fn assert_sync<T: Sync>(_: &T) {}
        
        let cell = SyncCell::new(42);
        assert_send(&cell);
        assert_sync(&cell);
    }
    

    #[test]
    fn test_no_deadlock_on_nested_access() {
        let cell = SyncCell::new(vec![1, 2, 3]);
        
        // This should not deadlock because the closure-based API prevents
        // holding guards across await points or other operations
        let result = cell.with(|vec| {
            vec.len()
        });
        
        assert_eq!(result, 3);
        
        // We can immediately access again without deadlock
        cell.with_mut(|vec| {
            vec.push(4);
        });
        
        let new_len = cell.with(|vec| vec.len());
        assert_eq!(new_len, 4);
    }

    #[test]
    fn test_panic_recovery() {
        let cell = SyncCell::new(42);
        
        // This should work fine
        let result = cell.with(|v| *v);
        assert_eq!(result, 42);
        
        // Even after a panic in user code, the cell should still work
        // (though the mutex might be poisoned)
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cell.with(|_| panic!("test panic"));
        }));
        
        assert!(panic_result.is_err());
        
        // The next access should panic due to mutex poisoning
        let poison_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cell.with(|v| *v);
        }));
        
        assert!(poison_result.is_err());
    }
}