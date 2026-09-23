//! Purpose: Verify discovery precedence with private runtime files instead of host PATH changes.

use super::*;
use tempfile::TempDir;

#[test]
fn java_home_precedes_path_and_invalid_java_home_falls_back_to_path() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("jdk");
    let path = root.path().join("path");
    std::fs::create_dir_all(home.join("bin")).unwrap();
    std::fs::create_dir(&path).unwrap();
    let java = launch::executable("java");
    std::fs::write(home.join("bin").join(&java), "jdk").unwrap();
    std::fs::write(path.join(&java), "path").unwrap();
    let discover_java = |home: Option<&Path>| {
        discover(&["java"], "missing", [path.clone()].into_iter(), home).unwrap()
    };
    assert_eq!(discover_java(Some(&home)), home.join("bin").join(&java));
    assert_eq!(discover_java(None), path.join(&java));
    assert_eq!(discover_java(Some(Path::new("relative"))), path.join(&java));
    assert_eq!(
        discover_java(Some(&root.path().join("absent"))),
        path.join(java)
    );
}

#[test]
fn discovery_ignores_relative_path_entries_and_checks_all_runtime_candidates() {
    let root = TempDir::new().unwrap();
    let program = root.path().join(launch::executable("python"));
    std::fs::write(&program, "runtime").unwrap();
    let paths = [PathBuf::from("."), root.path().to_owned()];
    assert_eq!(
        discover(
            &["missing", "python"],
            "no Python",
            paths.into_iter(),
            Some(root.path())
        )
        .unwrap(),
        program
    );
    assert_eq!(
        discover(
            &["missing"],
            "no runtime",
            [root.path().to_owned()].into_iter(),
            None
        ),
        Err(InstallError::Missing("no runtime"))
    );
    assert_eq!(launch::executable("server.exe"), "server.exe");
}
