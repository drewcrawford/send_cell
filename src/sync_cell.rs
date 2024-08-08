
/**
A cell that can be sent between threads.
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
    You must verify the Sync requirements of the underlying type.
    */
    pub unsafe fn get(&self) -> &T {
        &self.0
    }
    /**
    Gets the underlying value mutably.


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

impl<T: Clone> SyncCell<T> {
    /**
    Clones the underlying data.

    # Safety
    You must verify the Sync requirements of the underlying type.  In particular, sending a clone
    to a different thread may not be safe.
    */
    pub unsafe fn unchecked_clone(&self) -> Self {
        SyncCell(self.0.clone())
    }
}

/*
Design note about traits.

In general, &self functions cannot be implemented in Safe rust.  This rules out Debug, Clone, Copy,
PartialEq, Eq, PartialOrd, Ord, Hash,AsRef.  An unsafe `unchecked_clone` is provided.  In general,
chain through calls to the unsafe fn `get()`.

Default,From can be implemented as they work on owning types.

AsMut can be implemented, since we have an exclusive reference.

DerefMut can't be implemented due to lack of deref type.

 */





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



