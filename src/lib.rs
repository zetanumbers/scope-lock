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
// TODO: #![warn(missing_docs)]
// TODO: trait UnwrapArgTuple
// TODO: rename

use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr,
};

use parking_lot::{RwLock, RwLockReadGuard};

mod core;

pub fn lock_scope<'env, F, T>(scope: F)
where
    F: for<'scope> FnOnce(&'scope Extender<'scope, 'env>) -> T,
{
    let rw_lock = RwLock::new(());
    let extender = Extender {
        rc: unsafe {
            mem::transmute::<ReferenceCounter<'_>, ReferenceCounter<'static>>(
                ReferenceCounter::new(&rw_lock),
            )
        },
        scope: PhantomData,
        env: PhantomData,
    };
    let guard = ReferenceGuard {
        extender: &extender,
    };
    scope(&extender);
    drop(guard);
}

pub struct Extender<'scope, 'env> {
    rc: ReferenceCounter<'static>,
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> Extender<'scope, 'env> {
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
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + '_>,
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + 'static>,
                >(ptr::NonNull::new_unchecked(RefOnce::into_raw(f))),
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

    // TODO: Add futures
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
struct ReferenceGuard<'scope, 'env> {
    extender: &'scope Extender<'scope, 'env>,
}

impl Drop for ReferenceGuard<'_, '_> {
    fn drop(&mut self) {
        // faster to not unlock and just drop
        mem::forget(self.extender.rc.counter.write());
    }
}

// TODO: Erase argument and output somehow too

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

pub struct ExtendedFnMut<I, O> {
    // TODO: Could make a single dynamicly sized struct
    func: ptr::NonNull<dyn FnMut(I) -> O + Send>,
    _reference_guard: Reference<'static>,
}

impl<I, O> ExtendedFnMut<I, O> {
    pub fn call(&mut self, input: I) -> O {
        (unsafe { self.func.as_mut() })(input)
    }
}

unsafe impl<I, O> Send for ExtendedFnMut<I, O> {}

#[repr(transparent)]
struct Once<T: ?Sized>(mem::ManuallyDrop<T>);

// TODO: Fix clippy warning
unsafe trait ObjectSafeFnOnce<I> {
    type Output;

    /// Call closure
    ///
    /// # Safety
    ///
    /// Must be called at most once.
    unsafe fn call_once(&mut self, input: I) -> Self::Output;
}

unsafe impl<F, I, O> ObjectSafeFnOnce<I> for Once<F>
where
    F: FnOnce(I) -> O,
{
    type Output = O;

    unsafe fn call_once(&mut self, input: I) -> Self::Output {
        mem::ManuallyDrop::take(&mut self.0)(input)
    }
}

// TODO: split into separate crate
// TODO: Pin support
// TODO: [StableDeref](https://docs.rs/stable_deref_trait)
pub struct RefOnce<'a, T: ?Sized> {
    slot: &'a mut Once<T>,
}

impl<'a, T> RefOnce<'a, T> {
    pub fn new(value: T, slot: &'a mut mem::MaybeUninit<T>) -> Self {
        slot.write(value);
        RefOnce {
            slot: unsafe { mem::transmute::<&'a mut mem::MaybeUninit<T>, &'a mut Once<T>>(slot) },
        }
    }

    pub fn into_inner(this: Self) -> T {
        let mut this = mem::ManuallyDrop::new(this);
        unsafe { mem::ManuallyDrop::take(&mut this.slot.0) }
    }
}

impl<T: ?Sized> RefOnce<'_, T> {
    // TODO: make public
    fn into_raw(this: Self) -> *mut Once<T> {
        ptr::addr_of_mut!(*this.slot)
    }
}

impl<T: ?Sized> Deref for RefOnce<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.slot.0
    }
}

impl<T: ?Sized> DerefMut for RefOnce<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slot.0
    }
}

impl<T: ?Sized> Borrow<T> for RefOnce<'_, T> {
    fn borrow(&self) -> &T {
        &self.slot.0
    }
}

impl<T: ?Sized> BorrowMut<T> for RefOnce<'_, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.slot.0
    }
}

impl<T: ?Sized> Drop for RefOnce<'_, T> {
    fn drop(&mut self) {
        unsafe { mem::ManuallyDrop::drop(&mut self.slot.0) }
    }
}

pub struct ExtendedFnOnce<I, O> {
    // TODO: Could make a single dynamicly sized struct
    func: ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send>,
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

// TODO: zero case test
// TODO: tests from rust std::thread::scope
