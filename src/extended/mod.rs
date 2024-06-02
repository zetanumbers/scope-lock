use core::{marker::PhantomData, mem};

use parking_lot::{RwLock, RwLockReadGuard};

pub mod func;
pub mod future;

pub struct Extender<'scope, 'env> {
    rc: ReferenceCounter<'static>,
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> Extender<'scope, 'env> {
    pub(crate) fn new(rw_lock: &RwLock<()>) -> Self {
        Extender {
            rc: unsafe {
                mem::transmute::<ReferenceCounter<'_>, ReferenceCounter<'static>>(
                    ReferenceCounter::new(rw_lock),
                )
            },
            scope: PhantomData,
            env: PhantomData,
        }
    }

    pub(crate) fn guard(&'scope self) -> ReferenceGuard<'scope, 'env> {
        ReferenceGuard { extender: self }
    }
}

struct ReferenceCounter<'a> {
    counter: &'a RwLock<()>,
}

type Reference<'a> = RwLockReadGuard<'a, ()>;

impl<'a> ReferenceCounter<'a> {
    const fn new(rw_lock: &'a RwLock<()>) -> Self {
        Self { counter: rw_lock }
    }

    fn acquire(&self) -> Reference<'_> {
        self.counter.read()
    }
}

/// Waits for true on drop
pub(crate) struct ReferenceGuard<'scope, 'env> {
    extender: &'scope Extender<'scope, 'env>,
}

impl Drop for ReferenceGuard<'_, '_> {
    fn drop(&mut self) {
        // faster to not unlock and just drop
        mem::forget(self.extender.rc.counter.write());
    }
}

struct UnsafeAssertSync<T>(T);
unsafe impl<T> Sync for UnsafeAssertSync<T> {}

struct UnsafeAssertSend<T>(T);
unsafe impl<T> Send for UnsafeAssertSend<T> {}
