use core::marker::PhantomData;

pub mod func;
pub mod future;
pub mod sync;

pub struct Extender<'scope, 'env> {
    rc: &'scope sync::ReferenceCounter,
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> Extender<'scope, 'env> {
    pub(crate) fn new(rc: &'scope sync::ReferenceCounter) -> Self {
        Extender {
            rc,
            scope: PhantomData,
            env: PhantomData,
        }
    }

    pub(crate) fn guard(&'scope self) -> sync::ReferenceCounterGuard<'scope> {
        self.rc.guard()
    }

    unsafe fn associate_reference<T>(&'scope self, inner: T) -> AssociateReference<T> {
        AssociateReference {
            inner,
            _reference_guard: unsafe { self.rc.acquire() },
        }
    }
}

struct AssociateReference<T> {
    inner: T,
    // drop reference last
    _reference_guard: sync::Reference,
}

struct UnsafeAssertSync<T>(T);
unsafe impl<T> Sync for UnsafeAssertSync<T> {}

struct UnsafeAssertSend<T>(T);
unsafe impl<T> Send for UnsafeAssertSend<T> {}
