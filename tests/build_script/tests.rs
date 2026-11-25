//! Purpose: Unit tests for the root build script's Windows icon compilation and directive emission.

use super::{compile_windows_icon, emit_icon_directive};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("ffm-build-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&dir).expect("create build test temp dir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn fake_rc_tool(dir: &Path, name: &str, ok: bool) -> PathBuf {
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    let path = dir.join(format!("{name}{suffix}"));
    let body = match (cfg!(windows), ok) {
        (true, true) => {
            "@echo off\r\nif not \"%1\"==\"/I\" exit /b 2\r\nif not \"%3\"==\"/FO\" exit /b 3\r\nexit /b 0\r\n"
        }
        (true, false) => "@echo off\r\nexit /b 1\r\n",
        (false, true) => "#!/bin/sh\n[ \"$1\" = \"/I\" ] && [ \"$3\" = \"/FO\" ]\n",
        (false, false) => "#!/bin/sh\nexit 1\n",
    };
    fs::write(&path, body).expect("write fake rc tool");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod fake rc tool");
    }
    path
}

fn run_emit_child(action: &str, res: &Path) -> String {
    let exe = env::current_exe().expect("current test binary");
    let output = Command::new(exe)
        .args(["--exact", "tests::child_entrypoint", "--nocapture"])
        .env("FFM_BUILD_TEST_ACTION", action)
        .env("FFM_BUILD_TEST_RES", res)
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn compile_windows_icon_tries_tools_in_order_and_handles_missing_tools() {
    let temp = TempDir::new();
    let res = temp.0.join("ffm.res");
    let assets = temp.0.join("assets");
    let rc = assets.join("ffm.rc");
    let fail = fake_rc_tool(&temp.0, "fail_rc", false);
    let pass = fake_rc_tool(&temp.0, "pass_rc", true);
    let fail_str = fail.to_string_lossy().into_owned();
    let pass_str = pass.to_string_lossy().into_owned();
    let missing = temp.0.join("missing_rc").to_string_lossy().into_owned();

    assert!(!compile_windows_icon(
        &[&missing, &fail_str],
        &assets,
        &res,
        &rc
    ));
    assert!(compile_windows_icon(
        &[&missing, &fail_str, &pass_str],
        &assets,
        &res,
        &rc
    ));
}

#[test]
fn emit_icon_directive_prints_link_arg_only_when_compiled() {
    let temp = TempDir::new();
    let res = temp.0.join("ffm.res");
    let expected = format!("cargo:rustc-link-arg-bin=ffm={}", res.display());

    emit_icon_directive(&res, false);
    let emitted = run_emit_child("emit_true", &res);
    assert!(emitted.contains(&expected), "stdout was:\n{emitted}");

    let skipped = run_emit_child("emit_false", &res);
    assert!(
        !skipped.contains("cargo:rustc-link-arg-bin=ffm="),
        "stdout was:\n{skipped}"
    );
}

#[test]
fn child_entrypoint() {
    let Ok(action) = env::var("FFM_BUILD_TEST_ACTION") else {
        return;
    };
    let res = PathBuf::from(env::var_os("FFM_BUILD_TEST_RES").expect("fixture resource path"));
    match action.as_str() {
        "emit_true" => emit_icon_directive(&res, true),
        "emit_false" => emit_icon_directive(&res, false),
        other => panic!("unexpected FFM_BUILD_TEST_ACTION: {other}"),
    }
}
