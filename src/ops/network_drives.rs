//! Purpose: Persist discovered network mounts in the user's private ffm.yml.
use std::io;
mod discover;
mod storage;
pub use storage::save_to;
#[cfg(test)]
mod tests;

pub fn start() {
    let result = std::thread::Builder::new()
        .name("ffm-network-drives".to_owned())
        .spawn(|| {
            if let Err(error) = refresh() {
                crate::diagnostics::log_error(&format!("network drives: {error}"));
            }
        });
    if let Err(error) = result {
        crate::diagnostics::log_error(&format!("network drive worker: {error}"));
    }
}
fn refresh() -> io::Result<()> {
    let Some(path) = crate::sessions::SessionStore::path() else {
        return Ok(());
    };
    save_to(&path, &discover::read()?)
}
