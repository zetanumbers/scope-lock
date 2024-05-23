//! # Scope lock
//!
//! ## Examples
//!
//! ```
//! use std::thread;
//!
//! let mut a = vec![1, 2, 3];
//! let mut x = 0;
//!
//! let f1 = &|()| {
//!     println!("hello from the first scoped thread");
//!     // We can borrow `a` here.
//!     dbg!(&a);
//! };
//! let f2 = &mut |()| {
//!     println!("hello from the second scoped thread");
//!     // We can even mutably borrow `x` here,
//!     // because no other threads are using it.
//!     x += a[0] + a[2];
//! };
//!
//! scope_lock::lock_scope(|e| {
//!     thread::spawn({
//!         let f = e.extend_fn(f1);
//!         move || f.call(())
//!     });
//!     thread::spawn({
//!         let mut f = e.extend_fn_mut(f2);
//!         move || f.call(())
//!     });
//!     println!("hello from the main thread");
//! });
//!
//! // After the scope, we can modify and access our variables again:
//! a.push(4);
//! assert_eq!(x, a.len());
//! ```
use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr,
    sync::{Condvar, Mutex},
};

// TODO: miri
pub fn lock_scope<'env, F, T>(scope: F)
where
    F: for<'scope> FnOnce(&'scope Extender<'scope, 'env>) -> T,
{
    let extender = Extender {
        dropped_sema: SyncWatch::new(0),
        scope: PhantomData,
        env: PhantomData,
    };
    let _guard = ZeroGuard::new(&extender.dropped_sema);
    scope(&extender);
}

pub struct Extender<'scope, 'env> {
    dropped_sema: SyncWatch<usize>,
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

// TODO: Add extend ref (mut)
impl<'scope, 'env> Extender<'scope, 'env> {
    pub fn extend_fn<F, I, O>(&'scope self, f: &'scope F) -> ExtendedFn<I, O>
    where
        F: Fn(I) -> O + Sync + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        self.dropped_sema.inc();
        unsafe {
            ExtendedFn {
                func: mem::transmute::<
                    ptr::NonNull<dyn Fn(I) -> O + Sync + '_>,
                    ptr::NonNull<dyn Fn(I) -> O + Sync + 'static>,
                >(ptr::NonNull::from(f)),
                dropped_sema: mem::transmute::<&SyncWatch<usize>, &'static SyncWatch<usize>>(
                    &self.dropped_sema,
                ),
            }
        }
    }

    pub fn extend_fn_mut<F, I, O>(&'scope self, f: &'scope mut F) -> ExtendedFnMut<I, O>
    where
        F: FnMut(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        self.dropped_sema.inc();
        unsafe {
            ExtendedFnMut {
                func: mem::transmute::<
                    ptr::NonNull<dyn FnMut(I) -> O + Send + '_>,
                    ptr::NonNull<dyn FnMut(I) -> O + Send + 'static>,
                >(ptr::NonNull::from(f)),
                dropped_sema: mem::transmute::<&SyncWatch<usize>, &'static SyncWatch<usize>>(
                    &self.dropped_sema,
                ),
            }
        }
    }

    pub fn extend_fn_once<F, I, O>(&'scope self, f: RefOnce<'scope, F>) -> ExtendedFnOnce<I, O>
    where
        F: FnOnce(I) -> O + Send + 'scope,
        I: Send + 'scope,
        O: Send + 'scope,
    {
        self.dropped_sema.inc();
        unsafe {
            ExtendedFnOnce {
                func: mem::transmute::<
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + '_>,
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + 'static>,
                >(ptr::NonNull::new_unchecked(RefOnce::into_raw(f))),
                dropped_flag: mem::transmute::<&SyncWatch<usize>, &'static SyncWatch<usize>>(
                    &self.dropped_sema,
                ),
            }
        }
    }
}

/// Waits for true on drop
struct ZeroGuard<'a> {
    sema: &'a SyncWatch<usize>,
}

impl<'a> ZeroGuard<'a> {
    fn new(sema: &'a SyncWatch<usize>) -> Self {
        Self { sema }
    }
}

impl Drop for ZeroGuard<'_> {
    fn drop(&mut self) {
        self.sema.wait_for(&0)
    }
}

#[derive(Default)]
struct SyncWatch<T> {
    lock: Mutex<T>,
    condvar: Condvar,
}

impl SyncWatch<usize> {
    fn inc(&self) {
        // TODO: Figure out poisoning
        *self.lock.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        // TODO: Remove bc Semaphore
        self.condvar.notify_all();
    }

    fn dec(&self) {
        // TODO: Figure out poisoning
        *self.lock.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        self.condvar.notify_all();
    }
}

impl<T> SyncWatch<T> {
    const fn new(init_val: T) -> Self {
        Self {
            lock: Mutex::new(init_val),
            condvar: Condvar::new(),
        }
    }

    fn wait_for<U>(&self, stop_on_eq: &U)
    where
        T: PartialEq<U>,
    {
        self.wait_with(|v| *v == *stop_on_eq)
    }

    fn wait_with<F>(&self, mut stop_wait: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut lock_res = self.lock.lock();
        loop {
            let lock = lock_res.unwrap_or_else(|e| e.into_inner());
            if stop_wait(&lock) {
                break;
            }
            lock_res = self.condvar.wait(lock);
        }
    }
}

// TODO: Erase argument and output somehow too

pub struct ExtendedFn<I, O> {
    // TODO: Could make a single dynamicly sized struct
    func: ptr::NonNull<dyn Fn(I) -> O + Sync>,
    dropped_sema: &'static SyncWatch<usize>,
}

impl<I, O> ExtendedFn<I, O> {
    pub fn call(&self, input: I) -> O {
        (unsafe { self.func.as_ref() })(input)
    }
}

impl<I, O> Drop for ExtendedFn<I, O> {
    fn drop(&mut self) {
        self.dropped_sema.dec();
    }
}

// Almost just a simple reference, so it is Send and Sync
unsafe impl<I, O> Send for ExtendedFn<I, O> {}
unsafe impl<I, O> Sync for ExtendedFn<I, O> {}

pub struct ExtendedFnMut<I, O> {
    // TODO: Could make a single dynamicly sized struct
    func: ptr::NonNull<dyn FnMut(I) -> O + Send>,
    dropped_sema: &'static SyncWatch<usize>,
}

impl<I, O> ExtendedFnMut<I, O> {
    pub fn call(&mut self, input: I) -> O {
        (unsafe { self.func.as_mut() })(input)
    }
}

impl<I, O> Drop for ExtendedFnMut<I, O> {
    fn drop(&mut self) {
        self.dropped_sema.dec();
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
    dropped_flag: &'static SyncWatch<usize>,
}

impl<I, O> ExtendedFnOnce<I, O> {
    pub fn call(self, input: I) -> O {
        let mut this = mem::ManuallyDrop::new(self);
        let out = unsafe { this.func.as_mut().call_once(input) };
        this.dropped_flag.dec();
        out
    }
}

impl<I, O> Drop for ExtendedFnOnce<I, O> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(self.func.as_ptr()) };
        self.dropped_flag.dec();
    }
}

unsafe impl<I, O> Send for ExtendedFnOnce<I, O> {}

// TODO: zero case test
