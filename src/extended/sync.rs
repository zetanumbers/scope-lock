use std::sync::{Condvar, Mutex};

const ONE_REFERENCE: usize = 2;
const WAITING_FLAG: usize = 1;

pub struct ReferenceCounter {
    counter: Mutex<usize>,
    condvar: Condvar,
}

impl ReferenceCounter {
    pub fn new() -> Self {
        Self {
            counter: Mutex::new(0),
            condvar: Condvar::new(),
        }
    }

    pub unsafe fn acquire(&self) -> Reference {
        let mut counter = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        let Some(new_counter) = counter.checked_add(ONE_REFERENCE) else {
            drop(counter);
            panic!("Overflow of extended references count")
        };
        *counter = new_counter;
        Reference { rc: self }
    }

    pub fn guard(&self) -> ReferenceCounterGuard {
        ReferenceCounterGuard { rc: self }
    }
}

pub struct Reference {
    rc: *const ReferenceCounter,
}

impl Drop for Reference {
    fn drop(&mut self) {
        // NOTE: establishes release ordering
        unsafe {
            let mut counter = (*self.rc).counter.lock().unwrap_or_else(|e| e.into_inner());
            let new_counter = *counter - ONE_REFERENCE;
            *counter = new_counter;
            // notify only if scope already waits
            if new_counter == WAITING_FLAG {
                (*self.rc).condvar.notify_one();
            }
        }
    }
}

unsafe impl Send for Reference {}
unsafe impl Sync for Reference {}

pub struct ReferenceCounterGuard<'a> {
    rc: &'a ReferenceCounter,
}

impl<'a> Drop for ReferenceCounterGuard<'a> {
    fn drop(&mut self) {
        // NOTE: establishes acquire ordering
        let mut counter = self.rc.counter.lock().unwrap_or_else(|e| e.into_inner());
        if *counter == 0 {
            return;
        }
        *counter |= WAITING_FLAG;
        loop {
            counter = self
                .rc
                .condvar
                .wait(counter)
                .unwrap_or_else(|e| e.into_inner());

            if *counter == WAITING_FLAG {
                return;
            }
        }
    }
}
