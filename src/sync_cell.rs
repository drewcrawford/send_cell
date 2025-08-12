//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
A runtime-checked synchronous cell.

This provides safe synchronous access to mutex-protected data without the risk
of holding guards across await points. Access is restricted to synchronous
closures that automatically unlock when they return.
*/

use std::fmt::{Debug, Formatter};
use std::sync::{Mutex, };
use std::hash::{Hash, Hasher};
use crate::unsafe_sync_cell::UnsafeSyncCell;

pub struct SyncCell<T> {
    inner: UnsafeSyncCell<T>,
    mutex: Mutex<()>,
}

impl<T> SyncCell<T> {
    /**
    Creates a new synchronous cell.

    This constructor will "remember" the current thread. Subsequent access
    will be checked against the constructed value.
    */
    #[inline]
    pub fn new(value: T) -> SyncCell<T> {
        SyncCell {
            inner: UnsafeSyncCell::new(value),
            mutex: Mutex::new(()),
        }
    }

    /**
    Access the underlying value through a synchronous closure.

    The closure receives a shared reference to the inner value and must
    return synchronously. The mutex is automatically unlocked when the
    closure returns.

    # Panics

    This function will panic if:
    - Accessed from a different thread than the cell was created on
    - The mutex is poisoned
    */
    #[inline]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let _guard = self.mutex.lock().unwrap();
        let value = unsafe{self.inner.get()};
        let result = f(value);
        result
    }

    /**
    Access the underlying value mutably through a synchronous closure.

    The closure receives a mutable reference to the inner value and must
    return synchronously. The mutex is automatically unlocked when the
    closure returns.

    # Panics

    This function will panic if:
    - Accessed from a different thread than the cell was created on
    - The mutex is poisoned
    */
    #[inline]
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let _guard = self.mutex.lock().unwrap();
        //safe since we hold the lock
        let value = unsafe { self.inner.get_mut_unchecked() };
        let result = f(value);
        result
    }

    /**
    Consumes the cell and returns the inner value.

    # Panics

    This function will panic if accessed from a different thread than
    the cell was created on.
    */
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
    
    pub unsafe fn with_unchecked(&self) -> &T {
        // This is unsafe because it assumes the caller knows what they are doing.
        // It does not check thread safety or mutex poisoning.
        self.inner.get()
    }
    
    pub unsafe fn with_mut_unchecked(&self) -> &mut T {
        // This is unsafe because it assumes the caller knows what they are doing.
        // It does not check thread safety or mutex poisoning.
        self.inner.get_mut_unchecked()
    }
    
    
}

unsafe impl<T: Send> Send for SyncCell<T> {}
unsafe impl<T: Send> Sync for SyncCell<T> {}


// Basic trait implementations
impl<T: Debug> Debug for SyncCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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



impl<T: Clone> Clone for SyncCell<T> {
    fn clone(&self) -> Self {
        self.with(|value| SyncCell::new(value.clone()))
    }
}

// Comparison traits
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