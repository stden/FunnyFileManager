//! Give generated Cucumber futures the same stack budget on Windows and Unix.

pub fn run(suite: impl FnOnce() + Send + 'static) {
    // Windows executable entry threads have a smaller stack than libtest workers.
    // Construct and poll the large generated runner on this dedicated test thread.
    std::thread::Builder::new()
        .name("cucumber".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(suite)
        .expect("start BDD runner")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}
