use std::panic::UnwindSafe;

use crate::RefOnce;

// TODO: derive macro
// TODO: impl pointers
unsafe trait Erase {
    type Erased: 'static;

    fn erase(self) -> Self::Erased;
    unsafe fn restore(erased: Self::Erased) -> Self;
}

unsafe impl<'a, T> Erase for &'a T {
    type Erased = usize;

    #[inline(always)]
    fn erase(self) -> Self::Erased {
        unsafe { self as *const T as usize }
    }

    #[inline(always)]
    unsafe fn restore(erased: Self::Erased) -> Self {
        &*(erased as *const T)
    }
}

unsafe impl<'a, T> Erase for &'a mut T {
    type Erased = usize;

    #[inline(always)]
    fn erase(self) -> Self::Erased {
        unsafe { self as *mut T as usize }
    }

    #[inline(always)]
    unsafe fn restore(erased: Self::Erased) -> Self {
        &mut *(erased as *mut T)
    }
}

unsafe impl<'a, T> Erase for RefOnce<'a, T> {
    type Erased = usize;

    #[inline(always)]
    fn erase(self) -> Self::Erased {
        <&mut _>::erase(self.slot)
    }

    #[inline(always)]
    unsafe fn restore(erased: Self::Erased) -> Self {
        RefOnce {
            slot: <&mut _>::restore(erased),
        }
    }
}

unsafe impl<T> Erase for Box<T> {
    type Erased = usize;

    fn erase(self) -> Self::Erased {
        unsafe { Box::into_raw(self) as usize }
    }

    unsafe fn restore(erased: Self::Erased) -> Self {
        Box::from_raw(erased as *mut T)
    }
}
