use super::*;

use std::borrow::Borrow;
use std::mem::ManuallyDrop;
use std::ptr;

/// Conversion from unsized to sized.
///
/// Similar to `ToOwned`, but by taking ownership rather than duplicating.
pub unsafe trait IntoOwned {
    /// The resulting `Sized` type after conversion.
    type Owned : Borrow<Self> + Take<Self>;

    /// Performs the conversion.
    ///
    /// # Safety
    ///
    /// This function takes ownership of the value and thus the value now represents uninitialized
    /// data. It is up to the user of this method to ensure the uninitialized data is not actually
    /// used. In particular, `drop()` must not be called, and this function can only be called at
    /// most once for a given `ManuallyDrop<Self>` instance.
    unsafe fn into_owned_unchecked(this: &mut ManuallyDrop<Self>) -> Self::Owned;
}

unsafe impl<T> IntoOwned for T {
    type Owned = T;

    unsafe fn into_owned_unchecked(this: &mut ManuallyDrop<Self>) -> Self::Owned {
        (this as *const _ as *const Self).read()
    }
}

unsafe impl<T> IntoOwned for [T] {
    type Owned = Vec<T>;

    unsafe fn into_owned_unchecked(this: &mut ManuallyDrop<[T]>) -> Self::Owned {
        let len = this.len();

        let mut r = Vec::<T>::with_capacity(len);

        ptr::copy_nonoverlapping(this.as_ptr(), r.as_mut_ptr(), len);
        r.set_len(len);

        r
    }
}

#[cfg(test)]
mod tests {
}
