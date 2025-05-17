use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr;
use core::task;

use crate::Extender;
use crate::extended::sync::Reference;
use crate::pointer_like::PointerPinUnforgotten;
use crate::pointer_like::erased_static::{fn_drop, fn_poll_unforgotten};

impl<'scope, 'env> Extender<'scope, 'env> {
    // TODO: pointer width extended value like extend_future_box
    pub fn future<'extended, P, O>(
        &'scope self,
        f: P,
    ) -> impl Future<Output = O> + Send + Sync + 'extended
    where
        'extended: 'scope,
        P: PointerPinUnforgotten + Send + 'scope,
        P::Pointee: Future<Output = O>,
        O: Send + 'extended,
    {
        let reference_guard = unsafe { self.rc.acquire() };

        struct Fut<T> {
            inner: T,
            // drop last
            _reference_guard: Reference,
        }
        unsafe impl<T> Send for Fut<T> {}
        unsafe impl<T> Sync for Fut<T> {}
        impl<T: Future> Future for Fut<T> {
            type Output = T::Output;

            fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
                unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner).poll(cx) }
            }
        }

        // It is sync since there's no way to interact with a reference to returned type
        Fut {
            inner: unsafe { extend_future_unchecked(f) },
            _reference_guard: reference_guard,
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

#[allow(deprecated)]
pub mod legacy {
    use core::future::Future;
    use core::pin::Pin;
    use core::ptr;
    use core::task;

    use crate::extended::sync::Reference;

    #[doc(hidden)]
    #[deprecated = "leftover legacy code"]
    pub struct ExtendedFuture<O> {
        // TODO: Could make a single dynamically sized struct
        pub(crate) func: ptr::NonNull<dyn Future<Output = O> + Send>,
        pub(crate) _reference_guard: Reference,
    }

    #[doc(hidden)]
    impl<O> Future for ExtendedFuture<O> {
        type Output = O;

        fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
            unsafe { Pin::new_unchecked(self.get_unchecked_mut().func.as_mut()) }.poll(cx)
        }
    }

    #[doc(hidden)]
    unsafe impl<O> Send for ExtendedFuture<O> where O: Send {}
}
