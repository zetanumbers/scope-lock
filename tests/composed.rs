use std::future::Future;

fn check_static_closure_async<F, O>(_: F)
where
    F: FnOnce(i32) -> O + 'static,
    O: Future + 'static,
{
}

#[test]
fn closure_async_boxed() {
    let a = 37;
    scope_lock::lock_scope(|e| {
        check_static_closure_async({
            e.fn_mut(Box::new(|b| {
                e.future(Box::new(async move {
                    dbg!(&a);
                    dbg!(a + b);
                }))
            }))
        });
    });
}

// TODO
// #[test]
// fn closure_async_ref() {
//     let a = 37;
//     let mut s0 = MaybeUninit::uninit();
//     let mut s1 = MaybeUninit::uninit();
//     scope_lock::lock_scope(|e| {
//         check_static_closure_async({
//             e.fn_mut(RefOnce::new(
//                 |b| {
//                     e.future(RefOnce::new(
//                         async move {
//                             dbg!(&a);
//                             dbg!(a + b);
//                         },
//                         &mut s1,
//                     ))
//                 },
//                 &mut s0,
//             ))
//         });
//     });
// }
