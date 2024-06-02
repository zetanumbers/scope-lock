//! `'static` versions of the most popular functions.
//!
//! This module contains a way to type erase popular functions for any requested type without using
//! dynamic dispatch. It overcomes limitaions of generic free functions' instances inheriting
//! lifetime from a generic arguments.
//!
use core::{
    future::Future,
    panic::{RefUnwindSafe, UnwindSafe},
    pin::Pin,
    task,
};

use super::{PointerDeref, PointerDerefMut, PointerIntoInner, PointerLike, PointerPinUnforgotten};

mod fn_;

/// Get `'static` function of drop on a `P` pointer type.
///
/// # Safety
///
/// Only valid pointers of the same original smart-pointer type `P` must be passed to the returned
/// closure otherwise causing undefined behaviour.
pub const unsafe fn fn_drop<P: PointerLike>(
) -> impl Fn(*mut ()) + Copy + Send + Sync + UnwindSafe + RefUnwindSafe + Unpin + 'static {
    |erased_ptr| drop(unsafe { P::from_ptr(erased_ptr as *mut P::Pointee) })
}

pub const unsafe fn fn_call<P, I, O>(
) -> impl Fn(*const (), I) -> O + Copy + Send + Sync + UnwindSafe + RefUnwindSafe + Unpin + 'static
where
    P: PointerDeref,
    P::Pointee: Fn(I) -> O,
{
    |erased_ptr, input| unsafe { (*(erased_ptr as *const P::Pointee))(input) }
}

pub const unsafe fn fn_call_mut<P, I, O>(
) -> impl Fn(*mut (), I) -> O + Copy + Send + Sync + UnwindSafe + RefUnwindSafe + Unpin + 'static
where
    P: PointerDerefMut,
    P::Pointee: FnMut(I) -> O,
{
    |erased_ptr, input| unsafe { (*(erased_ptr as *mut P::Pointee))(input) }
}

pub const unsafe fn fn_call_once<P, I, O>(
) -> impl Fn(*mut (), I) -> O + Copy + Send + Sync + UnwindSafe + RefUnwindSafe + Unpin + 'static
where
    P: PointerIntoInner,
    P::Pointee: FnOnce(I) -> O,
{
    |erased_ptr, input| (unsafe { P::from_ptr(erased_ptr as *mut P::Pointee) }).into_inner()(input)
}

pub const unsafe fn fn_poll_unforgotten<P, O>(
) -> impl Fn(*mut (), &mut task::Context<'_>) -> task::Poll<O>
       + Copy
       + Send
       + Sync
       + UnwindSafe
       + RefUnwindSafe
       + Unpin
       + 'static
where
    P: PointerPinUnforgotten,
    P::Pointee: Future<Output = O>,
{
    |erased_ptr, cx| unsafe { (Pin::new_unchecked(&mut *(erased_ptr as *mut P::Pointee))).poll(cx) }
}
