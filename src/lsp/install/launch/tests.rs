//! Purpose: Verify relocatable launch receipts and runtime version parsing.

use super::*;
use crate::lsp::install::environment::System;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn launch_receipts_are_relocated_and_keep_literal_server_arguments() {
    let root = TempDir::new().unwrap();
    crate::test_support::write_executable(&root.path().join("server"), b"binary").unwrap();
    let recipe = InstallRecipe::Command {
        steps: vec![],
        executable: "server".into(),
    };
    let launch = for_recipe(&recipe, root.path(), &AtomicBool::new(false), &System).unwrap();
    let settings = ServerSettings {
        args: vec!["--value=a & b".into()],
        ..ServerSettings::default()
    };
    let prepared = relocated(&launch, &settings, root.path()).unwrap();
    assert_eq!(Path::new(&prepared.command), root.path().join("server"));
    assert_eq!(prepared.args, ["--value=a & b"]);
    assert!(prepared.install.is_none());
    fs::remove_file(root.path().join("server")).unwrap();
    assert!(relocated(&launch, &settings, root.path()).is_err());
}

fn relocated(
    launch: &Launch,
    settings: &ServerSettings,
    root: &Path,
) -> Result<ServerSettings, InstallError> {
    launch.settings(settings, root, root, &System, &AtomicBool::new(false))
}

struct TwoJavas {
    old: PathBuf,
    current: PathBuf,
}

impl Environment for TwoJavas {
    fn program(&self, names: &[&str], missing: &'static str) -> Result<PathBuf, InstallError> {
        self.runtimes(names)
            .into_iter()
            .next()
            .ok_or(InstallError::Missing(missing))
    }

    fn run(&self, command: &mut Command, _stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        if command.get_program() == self.old {
            Ok(b"openjdk 17.0.1".to_vec())
        } else {
            Ok(b"openjdk 21.0.1".to_vec())
        }
    }

    fn runtimes(&self, names: &[&str]) -> Vec<PathBuf> {
        if names == ["java"] {
            vec![self.old.clone(), self.current.clone()]
        } else {
            Vec::new()
        }
    }
}

fn two_javas(root: &Path) -> (TwoJavas, PathBuf) {
    let old = root.join("old-java");
    let current = root.join("java-21");
    crate::test_support::write_executable(&old, b"old").unwrap();
    crate::test_support::write_executable(&current, b"current").unwrap();
    (
        TwoJavas {
            old,
            current: current.clone(),
        },
        current,
    )
}

fn jdtls_settings() -> ServerSettings {
    ServerSettings {
        command: "jdtls".into(),
        install: Some(InstallRecipe::Jdtls {
            url: "https://example.com/jdtls.tar.gz".into(),
            sha256: "ab".repeat(32),
        }),
        ..ServerSettings::default()
    }
}

#[test]
fn java_21_on_path_is_used_when_java_home_is_older() {
    let root = TempDir::new().unwrap();
    let (environment, current) = two_javas(root.path());
    assert_eq!(
        java(&AtomicBool::new(false), &environment).unwrap(),
        current
    );
}

#[test]
fn cached_jdtls_launch_uses_the_same_java_21_choice() {
    let root = TempDir::new().unwrap();
    let (environment, current) = two_javas(root.path());
    let launch = Launch {
        command: "ignored".into(),
        args: Vec::new(),
        configuration: None,
    };
    let prepared = launch
        .settings(
            &jdtls_settings(),
            root.path(),
            root.path(),
            &environment,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(prepared.command, current.to_string_lossy());
}

#[test]
fn java_versions_distinguish_current_runtimes_from_java_eight_and_invalid_output() {
    assert_eq!(java_major("openjdk 21.0.11 2026-04-21"), Some(21));
    assert_eq!(java_major("java 25-ea"), Some(25));
    assert_eq!(java_major("java 1.8.0"), Some(1));
    assert_eq!(java_major("unrecognized"), None);
    assert_eq!(java_major("java unknown"), None);
}

#[test]
fn a_directory_named_like_a_java_launcher_is_rejected() {
    let root = TempDir::new().unwrap();
    fs::create_dir_all(
        root.path()
            .join("plugins/org.eclipse.equinox.launcher_1.jar"),
    )
    .unwrap();
    assert_eq!(
        java_launcher(root.path()).unwrap_err(),
        InstallError::Recipe
    );
}

#[cfg(unix)]
#[test]
fn custom_servers_require_executable_permission() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempDir::new().unwrap();
    let path = root.path().join("server");
    fs::write(&path, "binary").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let recipe = InstallRecipe::Command {
        steps: vec![],
        executable: "server".into(),
    };
    assert!(matches!(
        for_recipe(&recipe, root.path(), &AtomicBool::new(false), &System),
        Err(InstallError::Recipe)
    ));
}
