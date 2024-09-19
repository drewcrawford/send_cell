use std::fmt::Debug;

/**
A cell that can be shared between threads, even when its underlying data cannot be.

*/
pub struct SyncCell<T>(T);

unsafe impl<T> Sync for SyncCell<T> {}

impl <T> SyncCell<T> {
    pub fn new(value: T) -> Self {
        SyncCell(value)
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
    For other cases, you may want to pair [get] with interior mutability.
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

impl<T> Debug for SyncCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //we can't use the value here since we can't guarantee it's safe to do so.
        //but we can use the type name
        f.debug_tuple("SyncCell")
            .field(&std::any::type_name::<T>())
            .finish()

    }
}

impl<T: Default> SyncCell<T> {
    /**
    Creates a new SyncCell with the default value.
    */
    pub fn default() -> Self {
        SyncCell(T::default())
    }
}

impl<T> From<T> for SyncCell<T> {
    fn from(value: T) -> Self {
        SyncCell(value)
    }
}

impl<T> AsMut<T> for SyncCell<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}



