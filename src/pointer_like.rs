use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use crate::RefOnce;

/// Trait to convert various kinds of smart pointers and references into
/// a single raw pointer. Allows to implement more generic API.
///
/// # Safety
///
/// Implementation of this trait must satisfy several safety requirements:
///
/// - A raw pointer returned from [`PointerLike::into_ptr`] and passed to
///   [`PointerLike::from_ptr`] must not be null;
/// - The target type shall not be [subtyped] or unsized unless explicitly
///   allowed otherwise;
/// - A raw pointer passed to [`Self::from_ptr`] shall be returned from
///   [`Self::into_ptr`] implementation of the same type unless explicitly
///   allowed;
/// - A raw pointer may be able to outlive lifetime of the original smart
///   pointer, so user shall consider such pointer invalid outside of the
///   original lifetime.
///
/// [subtyped]: https://doc.rust-lang.org/reference/subtyping.html
pub unsafe trait PointerLike {
    /// The type our pointer points at.
    type Pointee;

    /// Convert smart pointer into a raw pointer, possibly leaking it.
    ///
    /// # Safety
    ///
    /// Dereferencing the returned pointer in any manner, writing to it
    /// or reading from it is disallowed unless it is specified otherwise.
    fn into_ptr(self) -> *mut Self::Pointee;

    /// Convert a raw pointer back into a smart pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be one returned from [`Self::into_ptr`] unless
    /// explicitly allowed otherwise. Be careful raw pointer must not
    /// outlive original smart pointer's lifetime.
    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self;
}

/// Trait that allows to create immutable references to the pointed-at
/// object.
///
/// # Safety
///
/// Implementer must guarantee safety of creating, using and
/// keeping of immutable references from the raw pointer returned by
/// [`PointerLike::into_ptr`]. It must also be allowed to operate on
/// this raw pointer in the manner of an immutable reference.
pub unsafe trait PointerDeref: PointerLike + Deref<Target = Self::Pointee> {}

/// Trait that allows to create a mutable reference to the pointed-at
/// object.
///
/// # Safety
///
/// Implementer must guarantee safety of creating, using and
/// keeping of mutable reference from the raw pointer returned by
/// [`PointerLike::into_ptr`]. It must also be allowed to operate on
/// this raw pointer in the manner of a mutable reference.
pub unsafe trait PointerDerefMut: PointerDeref + DerefMut {}

/// Trait that allows to move pointed-at object out of a smart-pointer,
/// and then, presumably, deallocating the smart-pointer.
///
/// # Safety
///
/// Implementer must guarantee safety of moving out of the
/// original smart-pointer and a raw pointer returned from
/// [`PointerLike::into_ptr`] via the [`Self::into_inner`] method.
pub unsafe trait PointerIntoInner: PointerDerefMut + DerefMut {
    fn into_inner(self) -> Self::Pointee;
}

/// Trait that allows to create pinned mutable references to the
/// pointed-at object, when assuming that the original smart-pointer
/// won't leak.
///
/// Such specific definition of this trait is needed to ensure
/// [pin's drop guarantee] in the context of extending lifetimes. If
/// something is unclear, so please refer to implementations and their
/// documentation. Specifically [`RefOnce`] impl might give you enough
/// insight.
///
/// # Safety
///
/// Implementer must guarantee safety of creating, using and keeping
/// of **pinned** mutable reference from the raw pointer returned by
/// [`PointerLike::into_ptr`], if the original smart-pointer won't
/// leak. It must also be allowed to operate on this raw pointer in the
/// manner of a **pinned** mutable reference.
///
/// [pin's drop guarantee]: https://doc.rust-lang.org/nightly/std/pin/index.html#subtle-details-and-the-drop-guarantee
pub unsafe trait PointerPinUnforgotten: PointerLike + PointerDeref {}

unsafe impl<T> PointerLike for &T {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        (self as *const T).cast_mut()
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { &*ptr.cast_const() }
    }
}
unsafe impl<T> PointerDeref for &T {}

unsafe impl<T> PointerLike for &mut T {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        self
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { &mut *ptr }
    }
}
unsafe impl<T> PointerDeref for &mut T {}
unsafe impl<T> PointerDerefMut for &mut T {}
/// Same as [`PointerDerefMut`] implementation because of [`Unpin`]
unsafe impl<T: Unpin> PointerPinUnforgotten for &mut T {}

unsafe impl<T> PointerLike for Box<T> {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        Box::into_raw(self)
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { Box::from_raw(ptr) }
    }
}
unsafe impl<T> PointerDeref for Box<T> {}
unsafe impl<T> PointerDerefMut for Box<T> {}
unsafe impl<T> PointerIntoInner for Box<T> {
    fn into_inner(self) -> Self::Pointee {
        *self
    }
}
/// Safe because of [`Box::into_pin`]
unsafe impl<T> PointerPinUnforgotten for Box<T> {}

unsafe impl<T> PointerLike for alloc::rc::Rc<T> {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        alloc::rc::Rc::into_raw(self).cast_mut()
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { alloc::rc::Rc::from_raw(ptr.cast_const()) }
    }
}
unsafe impl<T> PointerDeref for alloc::rc::Rc<T> {}

unsafe impl<T> PointerLike for alloc::sync::Arc<T> {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        alloc::sync::Arc::into_raw(self).cast_mut()
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { alloc::sync::Arc::from_raw(ptr.cast_const()) }
    }
}
unsafe impl<T> PointerDeref for alloc::sync::Arc<T> {}

unsafe impl<Ptr: PointerDeref> PointerLike for Pin<Ptr> {
    type Pointee = Ptr::Pointee;

    fn into_ptr(self) -> *mut Self::Pointee {
        unsafe { Pin::into_inner_unchecked(self) }.into_ptr()
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { Pin::new_unchecked(Ptr::from_ptr(ptr)) }
    }
}
unsafe impl<Ptr: PointerDeref> PointerDeref for Pin<Ptr> {}
unsafe impl<Ptr: PointerDerefMut> PointerDerefMut for Pin<Ptr> where Ptr::Pointee: Unpin {}
unsafe impl<Ptr: PointerIntoInner> PointerIntoInner for Pin<Ptr>
where
    Ptr::Pointee: Unpin,
{
    fn into_inner(self) -> Self::Pointee {
        unsafe { Pin::into_inner_unchecked(self) }.into_inner()
    }
}
/// We are already pinned, so this is fine
unsafe impl<Ptr: PointerDerefMut> PointerPinUnforgotten for Pin<Ptr> {}

unsafe impl<'a, T> PointerLike for RefOnce<'a, T> {
    type Pointee = T;

    fn into_ptr(self) -> *mut Self::Pointee {
        RefOnce::into_raw(self)
    }

    unsafe fn from_ptr(ptr: *mut Self::Pointee) -> Self {
        unsafe { RefOnce::from_raw(ptr) }
    }
}
unsafe impl<T> PointerDeref for RefOnce<'_, T> {}
unsafe impl<T> PointerDerefMut for RefOnce<'_, T> {}
unsafe impl<T> PointerIntoInner for RefOnce<'_, T> {
    fn into_inner(self) -> Self::Pointee {
        RefOnce::into_inner(self)
    }
}
/// Note that `RefOnce::into_pin` implementation, mimicking the
/// [`Box::into_pin`] would be unsound because [`RefOnce`] stores object
/// within the [`std::mem::MaybeUninit`] slot allocated somewhere
/// (including on a stack) which would allow us to deallocate pinned
/// object without first calling drop on it, thus violating the [pin's
/// drop guarantee]. However we can safely assume that this pointer won't
/// be forgotten and drop will "eventually" run, so we are safe here.
///
/// [pin's drop guarantee]: https://doc.rust-lang.org/nightly/std/pin/index.html#subtle-details-and-the-drop-guarantee
unsafe impl<T> PointerPinUnforgotten for RefOnce<'_, T> {}
