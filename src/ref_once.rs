use core::borrow::{Borrow, BorrowMut};
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr;

#[repr(transparent)]
pub(crate) struct Once<T: ?Sized>(mem::ManuallyDrop<T>);

/// Object-safe FnOnce
///
/// # Safety
///
/// [`ObjectSafeFnOnce::call_once`] may be called at most once.
pub(crate) unsafe trait ObjectSafeFnOnce<I> {
    type Output;

    /// Call closure
    ///
    /// # Safety
    ///
    /// May be called at most once.
    unsafe fn call_once(&mut self, input: I) -> Self::Output;
}

unsafe impl<F, I, O> ObjectSafeFnOnce<I> for Once<F>
where
    F: FnOnce(I) -> O,
{
    type Output = O;

    unsafe fn call_once(&mut self, input: I) -> Self::Output {
        unsafe { mem::ManuallyDrop::take(&mut self.0)(input) }
    }
}

// TODO: split into separate crate
pub struct RefOnce<'a, T: ?Sized> {
    slot: &'a mut Once<T>,
}

impl<'a, T> RefOnce<'a, T> {
    pub fn new(value: T, slot: &'a mut mem::MaybeUninit<T>) -> Self {
        slot.write(value);
        RefOnce {
            slot: unsafe { mem::transmute::<&'a mut mem::MaybeUninit<T>, &'a mut Once<T>>(slot) },
        }
    }

    pub fn into_inner(this: Self) -> T {
        let mut this = mem::ManuallyDrop::new(this);
        unsafe { mem::ManuallyDrop::take(&mut this.slot.0) }
    }
}

impl<'a, T: ?Sized> RefOnce<'a, T> {
    /// Essentially leaks object as a pointer until the original
    /// [`RefOnce`] is restored via [`Self::from_raw`].
    pub fn into_raw(this: Self) -> *mut T {
        let this = mem::ManuallyDrop::new(this);
        (unsafe { ptr::addr_of!(this.slot).read() } as *mut Once<T>) as *mut T
    }

    /// Convert pointer returned from [`Self::into_raw`] back into
    /// [`RefOnce`].
    ///
    /// # Safety
    ///
    /// `ptr` must have been returned from [`Self::into_raw`]. New
    /// lifetime argument `'a` of [`RefOnce`] should not outlive old
    /// lifetime not to cause any undefined behaviour.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        RefOnce {
            slot: unsafe { &mut *(ptr as *mut Once<T>) },
        }
    }
}

impl<T: ?Sized> RefOnce<'_, T> {
    // TODO: make public
    pub(crate) fn into_raw_once(this: Self) -> *mut Once<T> {
        let this = mem::ManuallyDrop::new(this);
        unsafe { ptr::addr_of!(this.slot).read() }
    }
}

impl<T: ?Sized> Deref for RefOnce<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.slot.0
    }
}

impl<T: ?Sized> DerefMut for RefOnce<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slot.0
    }
}

impl<T: ?Sized> Borrow<T> for RefOnce<'_, T> {
    fn borrow(&self) -> &T {
        &self.slot.0
    }
}

impl<T: ?Sized> BorrowMut<T> for RefOnce<'_, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.slot.0
    }
}

impl<T: ?Sized> Drop for RefOnce<'_, T> {
    fn drop(&mut self) {
        unsafe { mem::ManuallyDrop::drop(&mut self.slot.0) }
    }
}
