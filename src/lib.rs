use std::{
    borrow::{Borrow, BorrowMut},
    mem,
    ops::{Deref, DerefMut},
    ptr,
    sync::{Condvar, Mutex},
};

pub fn lock_scope<F>(scope: F)
where
    F: for<'a> FnOnce(Extender<'a>),
{
    let dropped = SyncWatch::new(false);
    let _guard = IfGuard::new(&dropped);
    let extender = Extender {
        dropped_flag: &dropped,
    };
    scope(extender);
}

#[derive(Clone, Copy)]
pub struct Extender<'a> {
    dropped_flag: &'a SyncWatch<bool>,
}

impl<'a> Extender<'a> {
    pub fn extend_fn<F, I, O>(self, f: &F) -> ExtendedFn<I, O>
    where
        F: Fn(I) -> O + Sync + 'a,
    {
        unsafe {
            ExtendedFn {
                func: mem::transmute::<
                    ptr::NonNull<dyn Fn(I) -> O + Sync + '_>,
                    ptr::NonNull<dyn Fn(I) -> O + Sync + 'static>,
                >(ptr::NonNull::from(f)),
                dropped_flag: mem::transmute::<&SyncWatch<bool>, &'static SyncWatch<bool>>(
                    self.dropped_flag,
                ),
            }
        }
    }

    pub fn extend_fn_mut<F, I, O>(self, f: &mut F) -> ExtendedFnMut<I, O>
    where
        F: FnMut(I) -> O + Send + 'a,
    {
        unsafe {
            ExtendedFnMut {
                func: mem::transmute::<
                    ptr::NonNull<dyn FnMut(I) -> O + Send + '_>,
                    ptr::NonNull<dyn FnMut(I) -> O + Send + 'static>,
                >(ptr::NonNull::from(f)),
                dropped_flag: mem::transmute::<&SyncWatch<bool>, &'static SyncWatch<bool>>(
                    self.dropped_flag,
                ),
            }
        }
    }

    pub fn extend_fn_once<F, I, O>(self, f: RefOnce<'_, F>) -> ExtendedFnOnce<I, O>
    where
        F: FnOnce(I) -> O + Send + 'a,
    {
        unsafe {
            ExtendedFnOnce {
                func: mem::transmute::<
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + '_>,
                    ptr::NonNull<dyn ObjectSafeFnOnce<I, Output = O> + Send + 'static>,
                >(ptr::NonNull::new_unchecked(RefOnce::into_raw(f))),
                dropped_flag: mem::transmute::<&SyncWatch<bool>, &'static SyncWatch<bool>>(
                    self.dropped_flag,
                ),
            }
        }
    }
}

/// Waits for true on drop
struct IfGuard<'a> {
    flag: &'a SyncWatch<bool>,
}

impl<'a> IfGuard<'a> {
    fn new(flag: &'a SyncWatch<bool>) -> Self {
        Self { flag }
    }
}

impl Drop for IfGuard<'_> {
    fn drop(&mut self) {
        self.flag.wait_for(&true)
    }
}

#[derive(Default)]
struct SyncWatch<T> {
    lock: Mutex<T>,
    condvar: Condvar,
}

impl<T> SyncWatch<T> {
    const fn new(init_val: T) -> Self {
        Self {
            lock: Mutex::new(init_val),
            condvar: Condvar::new(),
        }
    }

    fn replace(&self, new_val: T) -> T {
        let mut lock = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let out = mem::replace(&mut *lock, new_val);
        self.condvar.notify_all();
        out
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
    dropped_flag: &'static SyncWatch<bool>,
}

impl<I, O> ExtendedFn<I, O> {
    pub fn call(&self, input: I) -> O {
        (unsafe { self.func.as_ref() })(input)
    }
}

impl<I, O> Drop for ExtendedFn<I, O> {
    fn drop(&mut self) {
        self.dropped_flag.replace(true);
    }
}

// Almost just a simple reference, so it is Send and Sync
unsafe impl<I, O> Send for ExtendedFn<I, O> {}
unsafe impl<I, O> Sync for ExtendedFn<I, O> {}

pub struct ExtendedFnMut<I, O> {
    // TODO: Could make a single dynamicly sized struct
    func: ptr::NonNull<dyn FnMut(I) -> O + Send>,
    dropped_flag: &'static SyncWatch<bool>,
}

impl<I, O> ExtendedFnMut<I, O> {
    pub fn call(&mut self, input: I) -> O {
        (unsafe { self.func.as_mut() })(input)
    }
}

impl<I, O> Drop for ExtendedFnMut<I, O> {
    fn drop(&mut self) {
        self.dropped_flag.replace(true);
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
    dropped_flag: &'static SyncWatch<bool>,
}

impl<I, O> ExtendedFnOnce<I, O> {
    pub fn call(self, input: I) -> O {
        let mut this = mem::ManuallyDrop::new(self);
        unsafe { this.func.as_mut().call_once(input) }
    }
}

impl<I, O> Drop for ExtendedFnOnce<I, O> {
    fn drop(&mut self) {
        self.dropped_flag.replace(true);
    }
}

unsafe impl<I, O> Send for ExtendedFnOnce<I, O> {}
