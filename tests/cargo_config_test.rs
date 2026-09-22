//! Keep local compiler caching opt-in and package builds in one artifact tree.

use std::{path::Path, process::Command};

const CARGO_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"));

#[test]
fn compiler_cache_wrapper_stays_opt_in_with_foreground_server_setting() {
    let settings = CARGO_CONFIG
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert!(settings.contains(&"SCCACHE_NO_DAEMON = { value = \"1\", force = true }"));
    assert!(
        settings
            .iter()
            .all(|line| !line.starts_with("rustc-wrapper")),
        "the project must not override a developer's compiler-cache choice"
    );
}

fn target_directory(package: &Path) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version=1", "--offline"])
        .current_dir(package)
        .output()
        .expect("read Cargo metadata");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata = String::from_utf8(output.stdout).expect("Cargo metadata is UTF-8");
    metadata
        .split_once("\"target_directory\":\"")
        .expect("target directory field")
        .1
        .split('"')
        .next()
        .expect("target directory value")
        .to_owned()
}

#[test]
fn packages_share_dependency_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = target_directory(root);
    assert!(!target.is_empty());
    assert_eq!(target, target_directory(&root.join("mcp")));
    assert_eq!(target, target_directory(&root.join("colorer-sys")));
}
