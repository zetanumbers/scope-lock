//! # Scope lock
//!
//! Scope lock allows you to extend lifetime for certain kind of objects like closures to use them
//! where larger lifetimes are required, like [`std::thread::spawn`]. Start from [`lock_scope`].
//!
//! ## Examples
//!
//! Using boxes (requires allocation):
//!
//! ```
#![doc = include_str!("../examples/boxed.rs")]
//! ```
//!
//! Using [`RefOnce`]:
//!
//! ```
#![doc = include_str!("../examples/ref_once.rs")]
//! ```
#![warn(
    unsafe_op_in_unsafe_fn,
    clippy::std_instead_of_core,
    clippy::alloc_instead_of_core
)]

// TODO: #![warn(missing_docs)]
// TODO: trait UnwrapArgTuple
// TODO: rename
// TODO: no_std support

// TODO: gate under a feature
extern crate alloc;

mod extended;
pub mod pointer_like;
mod ref_once;

pub use extended::Extender;
pub use extended::func::{extend_fn_mut_unchecked, extend_fn_once_unchecked, extend_fn_unchecked};
pub use extended::future::extend_future_unchecked;
pub use extended::future::legacy::ExtendedFuture;
pub use ref_once::RefOnce;

pub fn lock_scope<'env, F, T>(scope: F) -> T
where
    F: for<'scope> FnOnce(&'scope Extender<'scope, 'env>) -> T,
{
    let rw_lock = extended::sync::ReferenceCounter::new();
    let extender = Extender::new(&rw_lock);
    let _guard = extender.guard();
    scope(&extender)
}

// TODO: zero case test
// TODO: tests from rust std::thread::scope
