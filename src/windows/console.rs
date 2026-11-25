//! Purpose: The spelling of a Windows path that cmd.exe understands.

/// Convert filesystem paths to the ordinary spelling understood by cmd.exe.
pub fn console_directory(path: &std::path::Path) -> std::path::PathBuf {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Component, Prefix},
    };
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return path.to_path_buf();
    };
    let text: Vec<u16> = path.as_os_str().encode_wide().collect();
    // `\\?\C:\…` drops its `\\?\`; `\\?\UNC\server\…` trades `\\?\UNC\` for `\\`.
    let normal = match prefix.kind() {
        Prefix::VerbatimDisk(_) => text.get(4..).map(<[u16]>::to_vec),
        Prefix::VerbatimUNC(_, _) => text
            .get(8..)
            .map(|share| [[92, 92].as_slice(), share].concat()),
        _ => None,
    };
    normal.map_or_else(
        || path.to_path_buf(),
        |normal| OsString::from_wide(&normal).into(),
    )
}
