use std::future::Future;

fn check_static_closure_async<F, O>(_: F)
where
    F: FnOnce(i32) -> O + 'static,
    O: Future + 'static,
{
}

#[test]
fn closure_async() {
    let a = 37;
    scope_lock::lock_scope(|e| {
        check_static_closure_async({
            let f = e.extend_fn_once_box(|b| {
                e.extend_future_box(async move {
                    dbg!(&a);
                    dbg!(a + b);
                })
            });
            move |b| f(b)
        });
    });
}
