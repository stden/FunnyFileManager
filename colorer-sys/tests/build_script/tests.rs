//! Build discovery and linking exercised through the production owner.

mod discovery;
mod linking;
mod process;

use std::fs;
use std::io;
use std::path::Path;

fn write(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path.parent().expect("fixture file has a parent"))?;
    fs::write(path, [])
}

fn native_build(root: &Path) -> io::Result<()> {
    write(&root.join("src/colorer/ParserFactory.h"))?;
    write(&root.join("_build/src/colorer/common/Features.h"))?;
    write(&root.join("_build/src/libcolorer.a"))
}

fn compiler(root: &Path) -> io::Result<std::path::PathBuf> {
    let path = root.join(if cfg!(windows) {
        "compiler.cmd"
    } else {
        "compiler"
    });
    let script = if cfg!(windows) {
        "@echo off\r\necho %FFM_COLORER_LIBRARY%\r\nexit /b %FFM_COLORER_STATUS%\r\n"
    } else {
        "#!/bin/sh\nprintf '%s\\n' \"$FFM_COLORER_LIBRARY\"\nexit \"$FFM_COLORER_STATUS\"\n"
    };
    fs::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(path)
}
