use std::{mem::MaybeUninit, thread};

use scope_lock::RefOnce;

fn main() {
    let mut a = vec![1, 2, 3];
    let mut x = 0;

    let mut slots = (MaybeUninit::uninit(), MaybeUninit::uninit());

    scope_lock::lock_scope(|e| {
        thread::spawn({
            let f = e.fn_(RefOnce::new(
                |()| {
                    println!("hello from the first scoped thread");
                    // We can borrow `a` here.
                    dbg!(&a);
                },
                &mut slots.0,
            ));
            move || f(())
        });
        thread::spawn({
            let mut f = e.fn_mut(RefOnce::new(
                |()| {
                    println!("hello from the second scoped thread");
                    // We can even mutably borrow `x` here,
                    // because no other threads are using it.
                    x += a[0] + a[2];
                },
                &mut slots.1,
            ));
            move || f(())
        });
        println!("hello from the main thread");
    });

    // After the scope, we can modify and access our variables again:
    a.push(4);
    assert_eq!(x, a.len());
}
