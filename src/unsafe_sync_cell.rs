//SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Debug;

/**
A cell that can be shared between threads, even when its underlying data cannot be.

This cell type is appropriate for cases where the values is "not actually shared between threads",
but Rust thinks it is.

For cases where it might really be shared after all, consider using [crate::sync_cell].

*/
pub struct UnsafeSyncCell<T>(T);

unsafe impl<T> Sync for UnsafeSyncCell<T> {}

impl <T> UnsafeSyncCell<T> {
    /**
    Creates a new cell.
*/
    #[inline]
    pub fn new(value: T) -> Self {
        UnsafeSyncCell(value)
    }
    /**
    Gets the underlying value.

    # Safety
    You must guarantee that whatever you are doing with the value is actually threadsafe.
    */
    pub unsafe fn get(&self) -> &T {
        &self.0
    }
    /**
    Gets the underlying value mutably.

    This is safe because the borrowchecker guarantees we have the only reference.
    For other cases, you may want to pair [Self::get] with interior mutability.
    */
    pub fn get_mut(&mut self) -> &mut T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        &mut self.0
    }


    /**
    Consumes the SyncCell and returns the inner value.
    */
    pub fn into_inner(self) -> T {
        //I think this should be safe, because we are the only ones with access to the inner value?
        self.0
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

impl<T: Default> UnsafeSyncCell<T> {
    /**
    Creates a new SyncCell with the default value.
    */
    pub fn default() -> Self {
        UnsafeSyncCell(T::default())
    }
}

impl<T> From<T> for UnsafeSyncCell<T> {
    fn from(value: T) -> Self {
        UnsafeSyncCell(value)
    }
}

impl<T> AsMut<T> for UnsafeSyncCell<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}



