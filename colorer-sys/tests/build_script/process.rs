use super::super::{configure, has_library, link_dependencies, locate, Found};
use super::{compiler, native_build, write};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

mod library;

fn child(root: &Path, case: &str) -> io::Result<Command> {
    let mut command = Command::new(env::current_exe()?);
    command.args(["--exact", "tests::process::child_entrypoint", "--nocapture"]);
    command
        .env("FFM_COLORER_TEST_CASE", case)
        .env("FFM_COLORER_TEST_ROOT", root);
    command
        .env("HOME", root.join("home"))
        .env_remove("USERPROFILE");
    command
        .env_remove("COLORER_INCLUDE_DIR")
        .env_remove("COLORER_BUILD_DIR");
    command.env_remove("COLORER_LINK_LIBS");
    Ok(command)
}

fn successful_output(command: &mut Command) -> io::Result<String> {
    let Output {
        status,
        stdout,
        stderr,
    } = command.output()?;
    assert!(status.success(), "{}", String::from_utf8_lossy(&stderr));
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

#[test]
fn explicit_environment_selects_only_the_supplied_tree() -> io::Result<()> {
    let directory = TempDir::new()?;
    native_build(directory.path())?;
    let mut command = child(directory.path(), "locate")?;
    command.env("COLORER_INCLUDE_DIR", directory.path().join("src"));
    command.env("COLORER_BUILD_DIR", directory.path().join("_build"));
    let output = successful_output(&mut command)?;
    assert!(output.contains(&format!(
        "FOUND={}",
        directory.path().join("_build/src").display()
    )));
    Ok(())
}

#[test]
fn userprofile_is_used_without_home() -> io::Result<()> {
    let directory = TempDir::new()?;
    let engine = directory.path().join(".local/share/ffm/Colorer-library");
    native_build(&engine)?;
    let mut command = child(directory.path(), "locate")?;
    command
        .env_remove("HOME")
        .env("USERPROFILE", directory.path());
    let output = successful_output(&mut command)?;
    assert!(output.contains(&format!("FOUND={}", engine.join("_build/src").display())));
    Ok(())
}

#[test]
fn windows_link_output_follows_vcpkg_and_override_names() -> io::Result<()> {
    let directory = TempDir::new()?;
    let libraries = directory
        .path()
        .join("_build/vcpkg_installed/x64-windows/lib");
    fs::create_dir_all(&libraries)?;
    for override_names in [None, Some(" alpha, , beta, ")] {
        let mut command = child(directory.path(), "link")?;
        command
            .env("CARGO_CFG_TARGET_OS", "windows")
            .env("CARGO_CFG_TARGET_ENV", "msvc");
        if let Some(names) = override_names {
            command.env("COLORER_LINK_LIBS", names);
        }
        let output = successful_output(&mut command)?;
        assert!(output.contains(&format!(
            "cargo:rustc-link-search=native={}",
            libraries.display()
        )));
        let expected = if override_names.is_some() {
            "alpha"
        } else {
            "icuin"
        };
        assert!(output.contains(&format!("cargo:rustc-link-lib=dylib={expected}")));
        assert!(!output.contains("dylib=stdc++"));
        assert!(!output.contains("dylib=minizip"));
    }
    Ok(())
}

#[test]
fn windows_without_vcpkg_emits_no_phantom_search_path() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut command = child(directory.path(), "link")?;
    command
        .env("CARGO_CFG_TARGET_OS", "windows")
        .env("CARGO_CFG_TARGET_ENV", "msvc");
    let output = successful_output(&mut command)?;
    assert!(!output.contains("cargo:rustc-link-search"));
    assert!(output.contains("cargo:rustc-link-lib=dylib=icuin"));
    Ok(())
}

#[test]
fn missing_compiler_does_not_emit_an_unavailable_library() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut command = child(directory.path(), "link")?;
    command
        .env("CARGO_CFG_TARGET_OS", "linux")
        .env("CARGO_CFG_TARGET_ENV", "gnu");
    command.env("CC", directory.path().join("missing-compiler"));
    let output = successful_output(&mut command)?;
    assert!(output.contains("cargo:rustc-link-lib=dylib=stdc++"));
    assert!(!output.contains("cargo:rustc-link-lib=dylib=minizip"));
    Ok(())
}

#[test]
fn fallback_configuration_emits_no_native_link_directives() -> io::Result<()> {
    let directory = TempDir::new()?;
    let output = successful_output(&mut child(directory.path(), "fallback")?)?;
    assert!(!output.contains("cargo:rustc-cfg=colorer"));
    assert!(!output.contains("cargo:rustc-link"));
    Ok(())
}

#[test]
fn child_entrypoint() {
    let Ok(case) = env::var("FFM_COLORER_TEST_CASE") else {
        return;
    };
    let root = PathBuf::from(env::var_os("FFM_COLORER_TEST_ROOT").expect("child fixture root"));
    match case.as_str() {
        "fallback" => configure(None),
        "library" => assert_eq!(
            has_library("fixture"),
            env::var("FFM_COLORER_EXPECTED").expect("probe result") == "true"
        ),
        "locate" => {
            let found = locate().expect("isolated native engine");
            println!("FOUND={}", found.lib_dir.display());
        }
        "link" => link_dependencies(&Found {
            include: root.join("src"),
            generated: root.join("_build/src"),
            lib_dir: root.join("_build/src/Release"),
        }),
        _ => panic!("unknown fixture action"),
    }
}
