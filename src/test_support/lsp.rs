//! Purpose: Hand the Language servers dialog a fake machine from outside the
//! crate, where `LiveInput::lsp_probe` and `crate::lsp` cannot be named.

use std::path::Path;

use crate::models::App;

/// The file name LSP lookup finds on `PATH` for the program `name`:
/// `name.exe` on Windows, `name` elsewhere.
pub use crate::lsp::program_file;

/// Give the Language servers dialog a fake machine: `cache` and `bin_dirs`.
///
/// `cache` is the LSP install cache and `bin_dirs` the whole `PATH`, with no
/// `JAVA_HOME`: availability and completion then depend on the fixture alone,
/// never on the machine the test runs on.
pub fn set_lsp_probe(app: &mut App, cache: &Path, bin_dirs: &[&Path]) {
    let path = crate::complete::execs::join_paths(bin_dirs.iter().copied());
    app.live.lsp_probe = crate::lsp::Probe::new(Some(cache.to_path_buf()), path);
}
