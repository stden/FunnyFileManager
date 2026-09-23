//! Purpose: Deterministic package-manager boundary that produces realistic installation artifacts.

use super::{environment::Environment, launch, InstallError};
use std::{cell::RefCell, fs, io::Write, path::Path, process::Command, sync::atomic::AtomicBool};
use tempfile::TempDir;

pub(super) struct Packages {
    pub root: TempDir,
    pub calls: RefCell<Vec<Vec<String>>>,
    pub version: String,
    pub archive: Vec<u8>,
}

impl Packages {
    pub fn new() -> Self {
        let root = TempDir::new().unwrap();
        for name in ["node", "java", "python3", "curl"] {
            fs::write(root.path().join(launch::executable(name)), "runtime").unwrap();
        }
        write(root.path(), "node_modules/npm/bin/npm-cli.js", b"npm");
        Self {
            root,
            calls: RefCell::new(Vec::new()),
            version: "openjdk 21.0.11 2026-04-21".into(),
            archive: java_archive(),
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
        if args == ["--version"] {
            return Ok(self.version.as_bytes().to_vec());
        }
        let root = command.get_current_dir().unwrap();
        match args.get(1).map(String::as_str) {
            Some("venv") => {
                let python = launch::venv_python(root);
                fs::create_dir_all(python.parent().unwrap()).unwrap();
                fs::write(python, "python").unwrap();
            }
            Some("pip") => write(root, "venv/installed", args.last().unwrap().as_bytes()),
            Some("install") => npm(root),
            _ => {
                let output = args.iter().position(|arg| arg == "--output").unwrap();
                fs::write(&args[output + 1], &self.archive).unwrap();
            }
        }
        Ok(Vec::new())
    }
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
