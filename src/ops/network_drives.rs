//! Purpose: Persist discovered network mounts in the user's private ffm.yml.
use std::io;
mod discover;
mod storage;
pub use storage::save_to;
#[cfg(test)]
mod tests;

pub fn start() {
    start_with(
        |run| {
            std::thread::Builder::new()
                .name("ffm-network-drives".to_owned())
                .spawn(run)
                .map(|_| ())
        },
        refresh,
    );
}

fn start_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
    run_refresh: fn() -> io::Result<()>,
) {
    let result = spawn(Box::new(move || {
        if let Err(error) = run_refresh() {
            crate::diagnostics::log_error(&format!("network drives: {error}"));
        }
    }));
    if let Err(error) = result {
        crate::diagnostics::log_error(&format!("network drive worker: {error}"));
    }
}

fn refresh() -> io::Result<()> {
    refresh_at(crate::sessions::SessionStore::path())
}

fn refresh_at(path: Option<std::path::PathBuf>) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    save_to(&path, &discover::read()?)
}
