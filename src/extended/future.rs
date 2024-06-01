use core::{future::Future, mem, pin::Pin, ptr, task};

use crate::extended::Reference;
use crate::Extender;

impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn extend_future<F>(&'scope self, f: Pin<&'scope mut F>) -> ExtendedFuture<F::Output>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        unsafe {
            ExtendedFuture {
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

    /// Extend lifetime of a future. Use [`Box::into_pin`] to pin the future.
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

pub struct ExtendedFuture<O> {
    // TODO: Could make a single dynamically sized struct
    func: ptr::NonNull<dyn Future<Output = O> + Send>,
    _reference_guard: Reference<'static>,
}

impl<O> Future for ExtendedFuture<O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        unsafe { Pin::new_unchecked(self.get_unchecked_mut().func.as_mut()) }.poll(cx)
    }
}

unsafe impl<O> Send for ExtendedFuture<O> where O: Send {}
