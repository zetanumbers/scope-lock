use core::marker::PhantomData;
use core::mem;
use core::ptr;

use crate::extended::Reference;
use crate::pointer_like::erased_static::{fn_call, fn_call_mut, fn_call_once, fn_drop};
use crate::pointer_like::{PointerDeref, PointerDerefMut, PointerIntoInner};
use crate::{ref_once, Extender, RefOnce};

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
            f.inner.0 .0(i)
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
            f.inner.0 .0(i)
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
            f.inner.0 .0(i)
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

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn` is deprecated as it utilizes dynamic dispatch and works exclusivelly with references, use [`Extender::fn_`](#method.fn_) instead"
    )]
    pub fn extend_fn<F, I, O>(&'scope self, f: &'scope F) -> legacy::ExtendedFn<I, O>
    where
        F: Fn(I) -> O + Sync + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            legacy::ExtendedFn {
                func: mem::transmute::<
                    ptr::NonNull<dyn Fn(I) -> O + Sync + '_>,
                    ptr::NonNull<dyn Fn(I) -> O + Sync + 'static>,
                >(ptr::NonNull::from(f)),
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
            }
        }
    }

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn_box` is deprecated as it utilizes dynamic dispatch and requires allocation, use [`Extender::fn_`](#method.fn_) instead"
    )]
    pub fn extend_fn_box<F, I, O>(&'scope self, f: F) -> Box<dyn Fn(I) -> O + Send + Sync>
    where
        F: Fn(I) -> O + Send + Sync + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let f = AssociateReference {
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
                inner: f,
            };
            mem::transmute::<
                Box<dyn Fn(I) -> O + Send + Sync + 'scope>,
                Box<dyn Fn(I) -> O + Send + Sync>,
            >(Box::new(move |i| {
                let f = &f;
                (f.inner)(i)
            }))
        }
    }

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn_mut` is deprecated as it utilizes dynamic dispatch and works exclusivelly with mutable references, use [`Extender::fn_mut`](#method.fn_mut) instead"
    )]
    pub fn extend_fn_mut<F, I, O>(&'scope self, f: &'scope mut F) -> legacy::ExtendedFnMut<I, O>
    where
        F: FnMut(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            legacy::ExtendedFnMut {
                func: mem::transmute::<
                    ptr::NonNull<dyn FnMut(I) -> O + Send + '_>,
                    ptr::NonNull<dyn FnMut(I) -> O + Send + 'static>,
                >(ptr::NonNull::from(f)),
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
            }
        }
    }

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn_mut_box` is deprecated as it utilizes dynamic dispatch and requires allocation, use [`Extender::fn_mut`](#method.fn_mut) instead"
    )]
    pub fn extend_fn_mut_box<F, I, O>(&'scope self, f: F) -> Box<dyn FnMut(I) -> O + Send>
    where
        F: FnMut(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let mut f = AssociateReference {
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
                inner: f,
            };
            mem::transmute::<Box<dyn FnMut(I) -> O + Send + 'scope>, Box<dyn FnMut(I) -> O + Send>>(
                Box::new(move |i| {
                    let f = &mut f;
                    (f.inner)(i)
                }),
            )
        }
    }

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn_once` is deprecated as it utilizes dynamic dispatch and works exclusivelly with `RefOnce`, use [`Extender::fn_once`](#method.fn_once) instead"
    )]
    pub fn extend_fn_once<F, I, O>(
        &'scope self,
        f: RefOnce<'scope, F>,
    ) -> legacy::ExtendedFnOnce<I, O>
    where
        F: FnOnce(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            legacy::ExtendedFnOnce {
                func: mem::transmute::<
                    ptr::NonNull<dyn ref_once::ObjectSafeFnOnce<I, Output = O> + Send + '_>,
                    ptr::NonNull<dyn ref_once::ObjectSafeFnOnce<I, Output = O> + Send + 'static>,
                >(ptr::NonNull::new_unchecked(RefOnce::into_raw_once(f))),
                reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
            }
        }
    }

    #[deprecated(
        since = "0.2.5",
        note = "`extend_fn_once_box` is deprecated as it utilizes dynamic dispatch and requires allocation, use [`Extender::fn_once`](#method.fn_once) instead"
    )]
    pub fn extend_fn_once_box<F, I, O>(&'scope self, f: F) -> Box<dyn FnOnce(I) -> O + Send>
    where
        F: FnOnce(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let f = AssociateReference {
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
                inner: f,
            };
            mem::transmute::<Box<dyn FnOnce(I) -> O + Send + 'scope>, Box<dyn FnOnce(I) -> O + Send>>(
                Box::new(move |i| {
                    let f = f;
                    (f.inner)(i)
                }),
            )
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

pub(crate) mod legacy {
    use core::{mem, ptr};

    use crate::{extended::Reference, ref_once};

    pub struct ExtendedFn<I, O> {
        pub(crate) func: ptr::NonNull<dyn Fn(I) -> O + Sync>,
        // drop reference guard last
        pub(crate) _reference_guard: Reference<'static>,
    }

    impl<I, O> ExtendedFn<I, O> {
        pub fn call(&self, input: I) -> O {
            (unsafe { self.func.as_ref() })(input)
        }
    }

    // Almost just a simple reference, so it is Send and Sync
    unsafe impl<I, O> Send for ExtendedFn<I, O> {}
    unsafe impl<I, O> Sync for ExtendedFn<I, O> {}
    // FIXME: unsafe impl<I, O> Send for ExtendedFnMut<I, O> where I: Send, O: Send {}
    // FIXME: unsafe impl<I, O> Sync for ExtendedFnMut<I, O> where I: Send, O: Send {}

    pub struct ExtendedFnMut<I, O> {
        // TODO: Could make a single dynamically sized struct
        pub(crate) func: ptr::NonNull<dyn FnMut(I) -> O + Send>,
        pub(crate) _reference_guard: Reference<'static>,
    }

    impl<I, O> ExtendedFnMut<I, O> {
        pub fn call(&mut self, input: I) -> O {
            (unsafe { self.func.as_mut() })(input)
        }
    }

    unsafe impl<I, O> Send for ExtendedFnMut<I, O> {}
    // FIXME: unsafe impl<I, O> Send for ExtendedFnMut<I, O> where I: Send, O: Send {}

    pub struct ExtendedFnOnce<I, O> {
        // TODO: Could make a single dynamically sized struct
        pub(crate) func: ptr::NonNull<dyn ref_once::ObjectSafeFnOnce<I, Output = O> + Send>,
        pub(crate) reference_guard: Reference<'static>,
    }

    impl<I, O> ExtendedFnOnce<I, O> {
        pub fn call(self, input: I) -> O {
            let mut this = mem::ManuallyDrop::new(self);
            let _reference_guard = unsafe { ptr::read(&this.reference_guard) };
            unsafe { this.func.as_mut().call_once(input) }
        }
    }

    impl<I, O> Drop for ExtendedFnOnce<I, O> {
        fn drop(&mut self) {
            unsafe { ptr::drop_in_place(self.func.as_ptr()) };
        }
    }

    unsafe impl<I, O> Send for ExtendedFnOnce<I, O> {}
    // FIXME: unsafe impl<I, O> Send for ExtendedFnOnce<I, O> where I: Send, O: Send {}
}
