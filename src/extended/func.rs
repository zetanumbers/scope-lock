use core::mem;
use core::ptr;

use crate::extended::Reference;
use crate::{ref_once, Extender, RefOnce};

impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn extend_fn<F, I, O>(&'scope self, f: &'scope F) -> ExtendedFn<I, O>
    where
        F: Fn(I) -> O + Sync + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            ExtendedFn {
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

    pub fn extend_fn_box<F, I, O>(&'scope self, f: F) -> Box<dyn Fn(I) -> O + Send + Sync>
    where
        F: Fn(I) -> O + Send + Sync + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let reference_guard =
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire());
            mem::transmute::<
                Box<dyn Fn(I) -> O + Send + Sync + 'scope>,
                Box<dyn Fn(I) -> O + Send + Sync>,
            >(Box::new(move |i| {
                let _reference_guard = &reference_guard;
                f(i)
            }))
        }
    }

    pub fn extend_fn_mut<F, I, O>(&'scope self, f: &'scope mut F) -> ExtendedFnMut<I, O>
    where
        F: FnMut(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            ExtendedFnMut {
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

    pub fn extend_fn_mut_box<F, I, O>(&'scope self, mut f: F) -> Box<dyn FnMut(I) -> O + Send>
    where
        F: FnMut(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let reference_guard =
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire());
            mem::transmute::<Box<dyn FnMut(I) -> O + Send + 'scope>, Box<dyn FnMut(I) -> O + Send>>(
                Box::new(move |i| {
                    let _reference_guard = &reference_guard;
                    f(i)
                }),
            )
        }
    }

    pub fn extend_fn_once<F, I, O>(&'scope self, f: RefOnce<'scope, F>) -> ExtendedFnOnce<I, O>
    where
        F: FnOnce(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            ExtendedFnOnce {
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

    pub fn extend_fn_once_box<F, I, O>(&'scope self, f: F) -> Box<dyn FnOnce(I) -> O + Send>
    where
        F: FnOnce(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        unsafe {
            let reference_guard =
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire());
            mem::transmute::<Box<dyn FnOnce(I) -> O + Send + 'scope>, Box<dyn FnOnce(I) -> O + Send>>(
                Box::new(move |i| {
                    let _reference_guard = &reference_guard;
                    f(i)
                }),
            )
        }
    }
}

// TODO: Erase argument and output somehow too
pub struct ExtendedFn<I, O> {
    // TODO: Could make a single dynamically sized struct
    func: ptr::NonNull<dyn Fn(I) -> O + Sync>,
    _reference_guard: Reference<'static>,
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
    func: ptr::NonNull<dyn FnMut(I) -> O + Send>,
    _reference_guard: Reference<'static>,
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
    func: ptr::NonNull<dyn ref_once::ObjectSafeFnOnce<I, Output = O> + Send>,
    reference_guard: Reference<'static>,
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
