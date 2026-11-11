use super::{child, compiler, native_build, successful_output, write};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const WINDOWS_TOOLCHAIN: &str = concat!(
    "@echo off\r\n",
    ":loop\r\n",
    "if \"%~1\"==\"\" exit /b 0\r\n",
    "if \"%~1\"==\"-o\" (\r\n",
    "  type nul > \"%~2\"\r\n",
    "  shift\r\n",
    "  shift\r\n",
    "  goto loop\r\n",
    ")\r\n",
    "if \"%~x1\"==\".a\" type nul > \"%~1\"\r\n",
    "shift\r\n",
    "goto loop\r\n",
);

const UNIX_TOOLCHAIN: &str = concat!(
    "#!/bin/sh\n",
    "prev=\"\"\n",
    "for arg in \"$@\"; do\n",
    "  if [ \"$prev\" = \"-o\" ]; then : > \"$arg\"; fi\n",
    "  case \"$arg\" in *.a) : > \"$arg\" ;; esac\n",
    "  prev=\"$arg\"\n",
    "done\n",
    "exit 0\n",
);

fn fake_toolchain(root: &Path) -> io::Result<PathBuf> {
    let (name, script) = if cfg!(windows) {
        ("toolchain.cmd", WINDOWS_TOOLCHAIN)
    } else {
        ("toolchain", UNIX_TOOLCHAIN)
    };
    let path = root.join(name);
    fs::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(path)
}

#[test]
fn configured_engine_compiles_shim_and_emits_native_directives() -> io::Result<()> {
    let directory = TempDir::new()?;
    native_build(directory.path())?;
    fs::write(directory.path().join("shim.cpp"), b"int ffm_shim;\n")?;
    let out_dir = directory.path().join("out");
    fs::create_dir_all(&out_dir)?;
    let tool = fake_toolchain(directory.path())?;
    let mut command = child(directory.path(), "configure")?;
    command
        .current_dir(directory.path())
        .env("OUT_DIR", &out_dir)
        .env("TARGET", "x86_64-unknown-linux-gnu")
        .env("HOST", "x86_64-unknown-linux-gnu")
        .env("OPT_LEVEL", "0")
        .env("CARGO_CFG_TARGET_OS", "linux")
        .env("CARGO_CFG_TARGET_ENV", "gnu")
        .env("CXX", &tool)
        .env("AR", &tool)
        .env("CC", directory.path().join("missing-compiler"));
    let output = successful_output(&mut command)?;
    assert!(output.contains("cargo:rustc-cfg=colorer"));
    assert!(output.contains("cargo:rustc-link-lib=static=colorer"));
    Ok(())
}

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
