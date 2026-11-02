use super::{child, compiler, successful_output, write};
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn library_probe_requires_a_successful_compiler_and_an_existing_path() -> io::Result<()> {
    let directory = TempDir::new()?;
    let executable = compiler(directory.path())?;
    let archive = directory.path().join("libfixture.so");
    write(&archive)?;
    for (output, status, expected) in [
        (PathBuf::from("libfixture.so"), "0", "false"),
        (directory.path().join("missing"), "0", "false"),
        (archive.clone(), "1", "false"),
        (archive, "0", "true"),
    ] {
        let mut command = child(directory.path(), "library")?;
        command
            .env("CC", &executable)
            .env("FFM_COLORER_LIBRARY", output);
        command
            .env("FFM_COLORER_STATUS", status)
            .env("FFM_COLORER_EXPECTED", expected);
        successful_output(&mut command)?;
    }
    Ok(())
}

#[test]
fn unix_link_output_includes_an_available_minizip() -> io::Result<()> {
    let directory = TempDir::new()?;
    let executable = compiler(directory.path())?;
    let archive = directory.path().join("libminizip.so");
    write(&archive)?;
    let mut command = child(directory.path(), "link")?;
    command
        .env("CARGO_CFG_TARGET_OS", "linux")
        .env("CARGO_CFG_TARGET_ENV", "gnu");
    command
        .env("CC", executable)
        .env("FFM_COLORER_LIBRARY", archive)
        .env("FFM_COLORER_STATUS", "0");
    let output = successful_output(&mut command)?;
    assert!(output.contains("cargo:rustc-link-lib=dylib=minizip"));
    assert!(output.contains("cargo:rustc-link-lib=dylib=stdc++"));
    Ok(())
}
