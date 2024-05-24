use std::thread;

fn main() {
    let mut a = vec![1, 2, 3];
    let mut x = 0;

    let f1 = &|()| {
        println!("hello from the first scoped thread");
        // We can borrow `a` here.
        dbg!(&a);
    };
    let f2 = &mut |()| {
        println!("hello from the second scoped thread");
        // We can even mutably borrow `x` here,
        // because no other threads are using it.
        x += a[0] + a[2];
    };

    scope_lock::lock_scope(|e| {
        thread::spawn({
            let f = e.extend_fn(f1);
            move || f.call(())
        });
        thread::spawn({
            let mut f = e.extend_fn_mut(f2);
            move || f.call(())
        });
        println!("hello from the main thread");
    });

    // After the scope, we can modify and access our variables again:
    a.push(4);
    assert_eq!(x, a.len());
}
