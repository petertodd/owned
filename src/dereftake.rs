use std::mem::{self, ManuallyDrop};
use std::ops;
use std::slice;
use std::rc::Rc;

use super::*;

/// `Deref`, but for taking ownership.
pub unsafe trait DerefTake : ops::Deref {
    /// Takes ownership, consuming the container.
    fn deref_take(self) -> <Self::Target as IntoOwned>::Owned
        where Self::Target: IntoOwned;

    /// Takes ownership of an unsized type with the aid of a closure.
    ///
    /// The closure is called with an mutable reference to `ManuallyDrop<T>`. After the closure
    /// returns the memory occupied by the value will be deallocated, but `drop()` will *not* be
    /// called on the value itself.
    fn deref_take_unsized<F, R>(self, f: F) -> R
        where F: FnOnce(&mut ManuallyDrop<Self::Target>) -> R;
}

unsafe impl<T: ?Sized> DerefTake for Box<T> {
    fn deref_take(self) -> <Self::Target as IntoOwned>::Owned
        where Self::Target: IntoOwned
    {
        self.deref_take_unsized(|src| {
            unsafe { Self::Target::into_owned_unchecked(src) }
        })
    }

    fn deref_take_unsized<F, R>(self, f: F) -> R
        where F: FnOnce(&mut ManuallyDrop<Self::Target>) -> R
    {
        let ptr = Box::into_raw(self) as *mut ManuallyDrop<T>;

        unsafe {
            let mut this: Box<ManuallyDrop<T>> = Box::from_raw(ptr);
            f(&mut this)
        }
    }
}

unsafe impl<T> DerefTake for Vec<T> {
    fn deref_take(self) -> <Self::Target as IntoOwned>::Owned
        where Self::Target: IntoOwned
    {
        self.deref_take_unsized(|src| {
            unsafe { Self::Target::into_owned_unchecked(src) }
        })
    }

    fn deref_take_unsized<F, R>(mut self, f: F) -> R
        where F: FnOnce(&mut ManuallyDrop<Self::Target>) -> R
    {
        unsafe {
            let len = self.len();

            // Setting the len to 0 means a panic won't call drop on any of the contained values.
            self.set_len(0);
            let src: &mut [T] = slice::from_raw_parts_mut(self.as_mut_ptr(), len);
            f(mem::transmute(src))
        }
    }
}

unsafe impl<T> DerefTake for ManuallyDrop<T> {
    fn deref_take(self) -> <Self::Target as IntoOwned>::Owned
        where Self::Target: IntoOwned
    {
        self.deref_take_unsized(|src| {
            unsafe { Self::Target::into_owned_unchecked(src) }
        })
    }

    fn deref_take_unsized<F, R>(mut self, f: F) -> R
        where F: FnOnce(&mut ManuallyDrop<Self::Target>) -> R
    {
        f(&mut self)
    }
}

unsafe impl<T: Clone> DerefTake for Rc<T> {
    fn deref_take(self) -> <Self::Target as IntoOwned>::Owned
        where Self::Target: IntoOwned
    {
        self.deref_take_unsized(|src| {
            unsafe { Self::Target::into_owned_unchecked(src) }
        })
    }

    fn deref_take_unsized<F, R>(self, f: F) -> R
        where F: FnOnce(&mut ManuallyDrop<Self::Target>) -> R
    {
        // Convert the Rc so that drop won't be called on the contents
        let mut this: Rc<ManuallyDrop<T>> = unsafe { Rc::from_raw(Rc::into_raw(self) as *const _) };

        // Get unique ownership.
        //
        // ManuallyDrop<T> is a #[repr(C)] wrapper, so it doesn't matter that we're doing the clone
        // here rather than above.
        f(Rc::make_mut(&mut this))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use dropcheck::{DropCheck, DropToken};

    #[test]
    fn test_box() {
        let check = DropCheck::new();

        let (token, state) = check.pair();
        let boxed = Box::new(token);
        let _token = boxed.deref_take();
        assert!(state.is_not_dropped());
    }

    #[test]
    fn test_vec() {
        let check = DropCheck::new();

        let v = vec![check.token(); 100];
        assert!(check.none_dropped());

        let _v2: Vec<DropToken> = v.deref_take();
        assert!(check.none_dropped());
    }

    #[test]
    fn test_rc() {
        let check = DropCheck::new();

        let (t1, s1) = check.pair();

        let rc1 = Rc::new(t1);
        assert!(s1.is_not_dropped());

        // only one owner, so no need to drop
        let _t1 = rc1.deref_take();
        assert!(s1.is_not_dropped());

        let (t1, s1) = check.pair();
        let rc1 = Rc::new(t1);
        let rc2 = Rc::clone(&rc1);

        // two owners, so deref_take() had to clone
        let _t1_clone = rc1.deref_take();
        assert!(s1.is_not_dropped());

        // the original is effectively now owned by just rc2, so when we drop it s1 gets dropped
        drop(rc2);
        assert!(s1.is_dropped());
    }
}
