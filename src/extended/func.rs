use core::marker::PhantomData;
use core::mem;
use core::ptr;

use crate::Extender;
use crate::extended::Reference;
use crate::pointer_like::erased_static::{fn_call, fn_call_mut, fn_call_once, fn_drop};
use crate::pointer_like::{PointerDeref, PointerDerefMut, PointerIntoInner};

use super::AssociateReference;
use super::{UnsafeAssertSend, UnsafeAssertSync};

impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn fn_once<'extended, P, I, O>(
        &'scope self,
        f: P,
    ) -> impl FnOnce(I) -> O + Send + Sync + 'extended
    where
        'extended: 'scope,
        P: PointerIntoInner + Send,
        P::Pointee: FnOnce(I) -> O,
        I: Send + 'extended,
        O: Send + 'extended,
    {
        let f = AssociateReference {
            _reference_guard: unsafe {
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire())
            },
            // Sync since there's no way to interact with a reference to returned type
            inner: UnsafeAssertSync(UnsafeAssertSend(unsafe { extend_fn_once_unchecked(f) })),
        };
        move |i| {
            let f = f;
            f.inner.0.0(i)
        }
    }

    pub fn fn_mut<'extended, P, I, O>(
        &'scope self,
        f: P,
    ) -> impl FnMut(I) -> O + Send + Sync + 'extended
    where
        'extended: 'scope,
        P: PointerDerefMut + Send,
        P::Pointee: FnMut(I) -> O,
        I: Send + 'extended,
        O: Send + 'extended,
    {
        let mut f = AssociateReference {
            _reference_guard: unsafe {
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire())
            },
            // Sync since there's no way to interact with a reference to returned type
            inner: UnsafeAssertSync(UnsafeAssertSend(unsafe { extend_fn_mut_unchecked(f) })),
        };
        move |i| {
            let f = &mut f;
            f.inner.0.0(i)
        }
    }

    pub fn fn_<'extended, P, I, O>(&'scope self, f: P) -> impl Fn(I) -> O + Send + Sync + 'extended
    where
        'extended: 'scope,
        P: PointerDeref + Send,
        P::Pointee: Fn(I) -> O + Sync,
        I: Send + 'extended,
        O: Send + 'extended,
    {
        let f = AssociateReference {
            _reference_guard: unsafe {
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire())
            },
            inner: UnsafeAssertSync(UnsafeAssertSend(unsafe { extend_fn_unchecked(f) })),
        };
        move |i| {
            let f = &f;
            f.inner.0.0(i)
        }
    }

    pub fn fn_unsync<'extended, P, I, O>(&'scope self, f: P) -> impl Fn(I) -> O + Send + 'extended
    where
        'extended: 'scope,
        P: PointerDeref + Send,
        P::Pointee: Fn(I) -> O,
        I: Send + 'extended,
        O: Send + 'extended,
    {
        let f = AssociateReference {
            _reference_guard: unsafe {
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire())
            },
            inner: UnsafeAssertSend(unsafe { extend_fn_unchecked(f) }),
        };
        move |i| {
            let f = &f;
            f.inner.0(i)
        }
    }
}

pub unsafe fn extend_fn_once_unchecked<'a, F, I, O>(f: F) -> impl FnOnce(I) -> O + 'a
where
    F: PointerIntoInner,
    F::Pointee: FnOnce(I) -> O,
    I: 'a,
    O: 'a,
{
    let f = unsafe {
        ErasedFn {
            ptr: ptr::NonNull::new_unchecked(f.into_ptr() as *mut ()),
            call: fn_call_once::<F, I, O>(),
            drop: fn_drop::<F>(),
            _marker: PhantomData,
        }
    };
    move |i| f.call_once(i)
}

pub unsafe fn extend_fn_mut_unchecked<'a, F, I, O>(f: F) -> impl FnMut(I) -> O + 'a
where
    F: PointerDerefMut,
    F::Pointee: FnMut(I) -> O,
    I: 'a,
    O: 'a,
{
    let mut f = unsafe {
        ErasedFn {
            ptr: ptr::NonNull::new_unchecked(f.into_ptr() as *mut ()),
            call: fn_call_mut::<F, I, O>(),
            drop: fn_drop::<F>(),
            _marker: PhantomData,
        }
    };
    move |i| f.call_mut(i)
}

pub unsafe fn extend_fn_unchecked<'a, F, I, O>(f: F) -> impl Fn(I) -> O + 'a
where
    F: PointerDeref,
    F::Pointee: Fn(I) -> O,
    I: 'a,
    O: 'a,
{
    let f = unsafe {
        ErasedFn {
            ptr: ptr::NonNull::new_unchecked(f.into_ptr() as *mut ()),
            call: fn_call::<F, I, O>(),
            drop: fn_drop::<F>(),
            _marker: PhantomData,
        }
    };
    move |i| f.call(i)
}

struct ErasedFn<C, I, O, D: Fn(*mut ())> {
    ptr: ptr::NonNull<()>,
    drop: D,
    call: C,
    _marker: PhantomData<fn(I) -> O>,
}

impl<C, I, O, D: Fn(*mut ())> ErasedFn<C, I, O, D>
where
    C: Fn(*mut (), I) -> O,
{
    fn call_once(self, input: I) -> O {
        let this = mem::ManuallyDrop::new(self);
        (this.call)(this.ptr.as_ptr(), input)
    }

    fn call_mut(&mut self, input: I) -> O {
        (self.call)(self.ptr.as_ptr(), input)
    }
}

impl<C, I, O, D: Fn(*mut ())> ErasedFn<C, I, O, D>
where
    C: Fn(*const (), I) -> O,
{
    fn call(&self, input: I) -> O {
        (self.call)(self.ptr.as_ptr(), input)
    }
}

impl<C, I, O, D: Fn(*mut ())> Drop for ErasedFn<C, I, O, D> {
    fn drop(&mut self) {
        (self.drop)(self.ptr.as_ptr())
    }
}
