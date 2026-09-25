//! Purpose: Deterministic package-manager boundary that produces realistic installation artifacts.

use super::{environment::Environment, launch, InstallError};
use std::{cell::RefCell, fs, io::Write, path::Path, process::Command, sync::atomic::AtomicBool};
use tempfile::TempDir;

pub(super) struct Packages {
    pub root: TempDir,
    pub calls: RefCell<Vec<Vec<String>>>,
    pub version: String,
    pub curl_version: String,
    pub archive: Vec<u8>,
    /// The step — `venv`, `pip`, `install` (npm) or `--output` (the
    /// download) — whose program exits with an error, if any.
    pub failing: Option<&'static str>,
}

impl Packages {
    pub fn new() -> Self {
        let root = TempDir::new().unwrap();
        for name in ["node", "java", "python3", "curl"] {
            crate::test_support::write_executable(
                &root.path().join(launch::executable(name)),
                b"runtime",
            )
            .unwrap();
        }
        write(root.path(), "node_modules/npm/bin/npm-cli.js", b"npm");
        Self {
            root,
            calls: RefCell::new(Vec::new()),
            version: "openjdk 21.0.11 2026-04-21".into(),
            curl_version: "curl 8.4.0 (test)".into(),
            archive: java_archive(),
            failing: None,
        }
    }
}

impl Environment for Packages {
    fn program(
        &self,
        names: &[&str],
        missing: &'static str,
    ) -> Result<std::path::PathBuf, InstallError> {
        names
            .iter()
            .map(|name| self.root.path().join(launch::executable(name)))
            .find(|path| path.is_file())
            .ok_or(InstallError::Missing(missing))
    }

    fn run(&self, command: &mut Command, _stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        self.calls.borrow_mut().push(args.clone());
        if let Some(answer) = canned(self, command, &args) {
            return answer;
        }
        let root = command.get_current_dir().unwrap();
        match args.get(1).map(String::as_str) {
            Some("venv") => {
                let python = launch::venv_python(root);
                fs::create_dir_all(python.parent().ok_or(InstallError::Recipe)?)?;
                crate::test_support::write_executable(&python, b"python")?;
            }
            Some("pip") => write(root, "venv/installed", args.last().unwrap().as_bytes()),
            Some("install") => npm(root),
            _ => {
                let output = args
                    .iter()
                    .position(|arg| arg == "--output")
                    .ok_or(InstallError::Recipe)?;
                fs::write(&args[output + 1], &self.archive).unwrap();
            }
        }
        Ok(Vec::new())
    }
}

/// What a `--version` call prints, or the error of the step set to fail;
/// `None` for a call that does its work.
fn canned(
    packages: &Packages,
    command: &Command,
    args: &[String],
) -> Option<Result<Vec<u8>, InstallError>> {
    if let Some(version) = version_output(packages, command, args) {
        return Some(Ok(version));
    }
    let failing = packages.failing?;
    args.iter()
        .any(|arg| arg == failing)
        .then_some(Err(InstallError::Command))
}

fn version_output(packages: &Packages, command: &Command, args: &[String]) -> Option<Vec<u8>> {
    if args != ["--version"] {
        return None;
    }
    let version = if Path::new(command.get_program()).file_stem().unwrap() == "curl" {
        &packages.curl_version
    } else {
        &packages.version
    };
    Some(version.as_bytes().to_vec())
}

fn npm(root: &Path) {
    write(
        root,
        "node_modules/server/package.json",
        br#"{"bin":{"server":"bin/main.js"}}"#,
    );
    write(root, "node_modules/server/bin/main.js", b"stdio");
}

pub(super) fn write(root: &Path, name: &str, contents: &[u8]) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn java_archive() -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    for name in [
        "plugins/org.eclipse.equinox.launcher_1.jar",
        "config_win/config.ini",
        "config_linux/config.ini",
        "config_mac/config.ini",
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, name, &b"data"[..])
            .unwrap();
    }
    let mut gzip = archive.into_inner().unwrap();
    gzip.flush().unwrap();
    gzip.finish().unwrap()
}
