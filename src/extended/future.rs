use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::ptr;
use core::task;

use crate::extended::Reference;
use crate::pointer_like::erased_static::{fn_drop, fn_poll_unforgotten, fn_poll_unpin};
use crate::pointer_like::{PointerDerefMut, PointerPinUnforgotten};
use crate::Extender;

impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn extend_future<F>(
        &'scope self,
        f: Pin<&'scope mut F>,
    ) -> legacy::ExtendedFuture<F::Output>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        unsafe {
            legacy::ExtendedFuture {
                func: mem::transmute::<
                    ptr::NonNull<dyn Future<Output = F::Output> + Send + '_>,
                    ptr::NonNull<dyn Future<Output = F::Output> + Send + 'static>,
                >(ptr::NonNull::from(f.get_unchecked_mut())),
                _reference_guard: mem::transmute::<Reference<'_>, Reference<'static>>(
                    self.rc.acquire(),
                ),
            }
        }
    }

    /// Extend lifetime of a future.
    pub fn extend_future_box<F>(
        &'scope self,
        f: F,
    ) -> Pin<Box<dyn Future<Output = F::Output> + Send>>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        unsafe {
            let reference_guard =
                mem::transmute::<Reference<'_>, Reference<'static>>(self.rc.acquire());
            Box::into_pin(mem::transmute::<
                Box<dyn Future<Output = F::Output> + Send + 'scope>,
                Box<dyn Future<Output = F::Output> + Send>,
            >(Box::new(async move {
                let _reference_guard = &reference_guard;
                f.await
            })))
        }
    }
}

pub unsafe fn extend_future_unchecked<'a, F, O>(f: F) -> impl Future<Output = O> + 'a
where
    F: PointerPinUnforgotten,
    F::Pointee: Future<Output = O>,
    O: 'a,
{
    unsafe {
        ErasedFuture {
            ptr: ptr::NonNull::new_unchecked(f.into_ptr() as *mut ()),
            poll: fn_poll_unforgotten::<F, O>(),
            drop: fn_drop::<F>(),
            _marker: PhantomData,
        }
    }
}

pub unsafe fn extend_future_unpin_unchecked<'a, F, O>(f: F) -> impl Future<Output = O> + 'a
where
    F: PointerDerefMut,
    F::Pointee: Future<Output = O> + Unpin,
    O: 'a,
{
    unsafe {
        ErasedFuture {
            ptr: ptr::NonNull::new_unchecked(f.into_ptr() as *mut ()),
            poll: fn_poll_unpin::<F, O>(),
            drop: fn_drop::<F>(),
            _marker: PhantomData,
        }
    }
}

struct ErasedFuture<P, O, D: Fn(*mut ())> {
    ptr: ptr::NonNull<()>,
    drop: D,
    poll: P,
    _marker: PhantomData<fn() -> task::Poll<O>>,
}

impl<P, O, D: Fn(*mut ())> Future for ErasedFuture<P, O, D>
where
    P: Fn(*mut (), &mut task::Context<'_>) -> task::Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        (self.poll)(self.ptr.as_ptr(), cx)
    }
}

impl<P, O, D: Fn(*mut ())> Drop for ErasedFuture<P, O, D> {
    fn drop(&mut self) {
        (self.drop)(self.ptr.as_ptr())
    }
}

pub mod legacy {
    use core::future::Future;
    use core::pin::Pin;
    use core::ptr;
    use core::task;

    use crate::extended::Reference;

    pub struct ExtendedFuture<O> {
        // TODO: Could make a single dynamically sized struct
        pub(crate) func: ptr::NonNull<dyn Future<Output = O> + Send>,
        pub(crate) _reference_guard: Reference<'static>,
    }

    impl<O> Future for ExtendedFuture<O> {
        type Output = O;

        fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
            unsafe { Pin::new_unchecked(self.get_unchecked_mut().func.as_mut()) }.poll(cx)
        }
    }

    unsafe impl<O> Send for ExtendedFuture<O> where O: Send {}
}
