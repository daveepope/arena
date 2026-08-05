use futures::channel::oneshot;

pub(crate) fn run_blocking<F, T>(f: F) -> impl std::future::Future<Output = T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel::<std::thread::Result<T>>();
    std::thread::spawn(move || {
        let _ = tx.send(std::panic::catch_unwind(f));
    });
    async move {
        match rx.await {
            Ok(Ok(v)) => v,
            Ok(Err(payload)) => std::panic::resume_unwind(payload),
            Err(_) => panic!("arena-postgres: blocking worker thread unexpectedly stopped"),
        }
    }
}
