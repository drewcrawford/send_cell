use std::fmt::Debug;

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

