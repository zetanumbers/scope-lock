//! # Scope lock
//!
//! Scope lock allows you to extend lifetime for certain kind of objects like closures to use them
//! where larger lifetimes are required, like [`std::thread::spawn`]. Start from [`lock_scope`].
//!
//! ## Examples
//!
//! Using boxes (requires allocation)
//!
//! ```
#![doc = include_str!("../examples/boxed.rs")]
//! ```
//!
//! Using references
//!
//! ```
#![doc = include_str!("../examples/references.rs")]
//! ```
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::std_instead_of_core, clippy::alloc_instead_of_core)]

// TODO: #![warn(missing_docs)]
// TODO: trait UnwrapArgTuple
// TODO: rename
// TODO: no_std support

// TODO: gate under a feature
extern crate alloc;

use parking_lot::RwLock;

mod extended;
pub mod pointer_like;
mod ref_once;

pub use extended::func::{ExtendedFn, ExtendedFnMut, ExtendedFnOnce};
pub use extended::future::ExtendedFuture;
pub use extended::Extender;
pub use ref_once::RefOnce;

pub fn lock_scope<'env, F, T>(scope: F)
where
    F: for<'scope> FnOnce(&'scope Extender<'scope, 'env>) -> T,
{
    let rw_lock = RwLock::new(());
    let extender = Extender::new(&rw_lock);
    let guard = extender.guard();
    scope(&extender);
    drop(guard);
}

// TODO: zero case test
// TODO: tests from rust std::thread::scope
