//! Purpose: Resolve installed server entry points and workspace-specific Java launch arguments.

use super::{environment::Environment, storage, InstallError};
use crate::config::{InstallRecipe, ServerSettings};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

#[derive(Serialize, Deserialize)]
pub(super) struct Launch {
    command: String,
    args: Vec<String>,
    configuration: Option<String>,
}

mod receipt;

pub(super) fn for_recipe(
    recipe: &InstallRecipe,
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<Launch, InstallError> {
    match recipe {
        InstallRecipe::Python { module, .. } => {
            let executable = venv_python(root);
            installed(&executable)?;
            Ok(Launch {
                command: template(root, &executable)?,
                args: vec!["-m".into(), module.clone()],
                configuration: None,
            })
        }
        InstallRecipe::Npm {
            package, binary, ..
        } => node(root, package, binary, environment),
        InstallRecipe::Jdtls { .. } => jdtls(root, stop, environment),
        InstallRecipe::Command { executable, .. } => {
            let path = storage::relative(root, executable)?;
            if !runnable(&path) {
                return Err(InstallError::Recipe);
            }
            Ok(Launch {
                command: template(root, &path)?,
                args: Vec::new(),
                configuration: None,
            })
        }
    }
}

fn installed(path: &Path) -> Result<(), InstallError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(InstallError::Recipe)
    }
}

fn template(root: &Path, path: &Path) -> Result<String, InstallError> {
    let relative = path.strip_prefix(root).map_err(|_| InstallError::Recipe)?;
    Ok(format!("{{install_dir}}/{}", relative.to_string_lossy()))
}

fn node(
    root: &Path,
    package: &str,
    binary: &str,
    environment: &dyn Environment,
) -> Result<Launch, InstallError> {
    let node = environment.program(&["node"], "LSP installation requires Node.js")?;
    let package = storage::relative(&root.join("node_modules"), package)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(package.join("package.json"))?)
            .map_err(|_| InstallError::Recipe)?;
    let bin = manifest["bin"]
        .as_str()
        .or_else(|| manifest["bin"][binary].as_str())
        .ok_or(InstallError::Recipe)?;
    let script = storage::relative(&package, bin)?;
    installed(&script)?;
    Ok(Launch {
        command: node.to_string_lossy().into_owned(),
        args: vec![template(root, &script)?],
        configuration: None,
    })
}

fn jdtls(
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<Launch, InstallError> {
    let java = java(stop, environment)?;
    let launcher = java_launcher(root)?;
    let configuration = format!("{{install_dir}}/{}/config.ini", java_configuration());
    installed(&root.join(java_configuration()).join("config.ini"))?;
    Ok(Launch {
        command: java.to_string_lossy().into_owned(),
        args: JAVA_ARGS
            .iter()
            .map(|arg| (*arg).to_owned())
            .chain([
                template(root, &launcher)?,
                "-configuration".into(),
                "{workspace_data}/config".into(),
                "-data".into(),
                "{workspace_data}/data".into(),
            ])
            .collect(),
        configuration: Some(configuration),
    })
}

fn java_launcher(root: &Path) -> Result<PathBuf, InstallError> {
    for entry in fs::read_dir(root.join("plugins"))? {
        let path = entry?.path();
        if path.is_file()
            && path.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with("org.eclipse.equinox.launcher_")
            })
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
        {
            return Ok(path);
        }
    }
    Err(InstallError::Recipe)
}

const JAVA_ARGS: &[&str] = &[
    "-Declipse.application=org.eclipse.jdt.ls.core.id1",
    "-Dosgi.bundles.defaultStartLevel=4",
    "-Declipse.product=org.eclipse.jdt.ls.core.product",
    "-Dosgi.install.area={install_dir}",
    "-Xmx1G",
    "--add-modules=ALL-SYSTEM",
    "--add-opens=java.base/java.util=ALL-UNNAMED",
    "--add-opens=java.base/java.lang=ALL-UNNAMED",
    "-jar",
];

pub(super) const JAVA_REQUIRED: &str = "Java LSP requires Java 21+ (set JAVA_HOME)";

pub(super) fn java(
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<PathBuf, InstallError> {
    for candidate in environment.runtimes(&["java"]) {
        if acceptable_java(&candidate, stop, environment)? {
            return Ok(candidate);
        }
    }
    Err(InstallError::Missing(JAVA_REQUIRED))
}

fn acceptable_java(
    path: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<bool, InstallError> {
    let version = match environment.run(Command::new(path).arg("--version"), stop) {
        Ok(version) => version,
        Err(error @ (InstallError::Cancelled | InstallError::Timeout)) => return Err(error),
        Err(_) => return Ok(false),
    };
    Ok(java_major(&String::from_utf8_lossy(&version)).is_some_and(|version| version >= 21))
}

fn java_major(version: &str) -> Option<u32> {
    version
        .split_whitespace()
        .nth(1)?
        .split(['.', '-'])
        .next()?
        .parse()
        .ok()
}

pub(super) fn find_all(names: &[&str], paths: impl Iterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in paths.filter(|path| path.is_absolute()) {
        for name in names {
            let path = directory.join(executable(name));
            if runnable(&path) {
                found.push(path);
            }
        }
    }
    found
}

#[cfg(unix)]
pub(super) fn runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
pub(super) fn runnable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
pub(super) fn executable(name: &str) -> String {
    if Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        name.to_owned()
    } else {
        format!("{name}.exe")
    }
}
#[cfg(unix)]
pub(super) fn executable(name: &str) -> String {
    name.to_owned()
}
#[cfg(windows)]
pub(super) fn venv_python(root: &Path) -> PathBuf {
    root.join("venv/Scripts/python.exe")
}
#[cfg(unix)]
pub(super) fn venv_python(root: &Path) -> PathBuf {
    root.join("venv/bin/python")
}
#[cfg(windows)]
fn java_configuration() -> &'static str {
    "config_win"
}
#[cfg(target_os = "macos")]
fn java_configuration() -> &'static str {
    "config_mac"
}
#[cfg(all(unix, not(target_os = "macos")))]
fn java_configuration() -> &'static str {
    "config_linux"
}

#[cfg(test)]
mod tests;
