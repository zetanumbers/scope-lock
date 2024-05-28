use crate::Extender;

mod erase;

impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn extend_fn<C, I, O>(&'scope self, f: fn(C, I) -> O, capture: C) -> ExtendedFn<I, O>
    where
        C: Send + 'scope,
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
}

pub struct ExtendedFn<I, O> {
    // TODO: Could make a single dynamicly sized struct
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
