//! Purpose: Launching the Eclipse JDT language server — a Java 21+ runtime,
//! the Equinox launcher jar and this platform's configuration directory.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

use super::{installed, template, Launch};
use crate::lsp::install::{environment::Environment, InstallError};

pub(super) fn jdtls(
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

pub(super) fn java_launcher(root: &Path) -> Result<PathBuf, InstallError> {
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

const JAVA_REQUIRED: &str = "Java LSP requires Java 21+ (set JAVA_HOME)";

/// Whether a `java` is there to try, found as [`java`] finds one but
/// without running it: whether it is 21+ is known only once it runs.
pub(in crate::lsp::install) fn java_present(
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    environment.program(&["java"], JAVA_REQUIRED).map(drop)
}

pub(in crate::lsp::install) fn java(
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

pub(super) fn java_major(version: &str) -> Option<u32> {
    version
        .split_whitespace()
        .nth(1)?
        .split(['.', '-'])
        .next()?
        .parse()
        .ok()
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
