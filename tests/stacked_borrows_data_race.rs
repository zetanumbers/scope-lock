use std::{mem, thread};

use scope_lock::RefOnce;

#[test]
fn ref_once_data_race() {
    let mut a = vec![1, 2, 3];
    let mut x = 0;

    {
        let mut f = |()| x += a[0] + a[2];
        scope_lock::lock_scope(|e| {
            let mut f = e.fn_mut(&mut f);
            thread::Builder::new()
                .name("first_spawned".into())
                .spawn(move || f(()))
                .unwrap();
        });
    }

    // After the scope, we can modify and access our variables again:
    a.push(4);
    assert_eq!(x, a.len());
}

#[test]
fn reference_data_race() {
    let mut a = vec![1, 2, 3];
    let mut x = 0;

    {
        let mut slot = mem::MaybeUninit::uninit();
        scope_lock::lock_scope(|e| {
            let mut f = e.fn_mut(RefOnce::new(|()| x += a[0] + a[2], &mut slot));
            thread::Builder::new()
                .name("first_spawned".into())
                .spawn(move || f(()))
                .unwrap();
        });
    }

    // After the scope, we can modify and access our variables again:
    a.push(4);
    assert_eq!(x, a.len());
}
