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

impl Launch {
    pub fn settings(
        &self,
        settings: &ServerSettings,
        root: &Path,
        workspace: &Path,
    ) -> Result<ServerSettings, InstallError> {
        let prepared = self.preview(settings, root, workspace)?;
        let data = workspace_data(root, workspace);
        if let Some(configuration) = &self.configuration {
            fs::create_dir_all(data.join("config"))?;
            let target = data.join("config/config.ini");
            if !target.exists() {
                fs::copy(expand(configuration, root, &data), target)?;
            }
        }
        Ok(prepared)
    }

    pub(super) fn preview(
        &self,
        settings: &ServerSettings,
        root: &Path,
        workspace: &Path,
    ) -> Result<ServerSettings, InstallError> {
        let data = workspace_data(root, workspace);
        let mut prepared = settings.clone();
        prepared.command = expand(&self.command, root, &data);
        installed(Path::new(&prepared.command))?;
        prepared.args = self
            .args
            .iter()
            .chain(&settings.args)
            .map(|arg| expand(arg, root, &data))
            .collect();
        prepared.install = None;
        Ok(prepared)
    }
}

fn workspace_data(root: &Path, workspace: &Path) -> PathBuf {
    root.join("workspaces")
        .join(storage::digest(workspace.as_os_str().as_encoded_bytes()))
}

fn expand(value: &str, root: &Path, data: &Path) -> String {
    value
        .replace("{install_dir}", &root.to_string_lossy())
        .replace("{workspace_data}", &data.to_string_lossy())
}

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
            installed(&path)?;
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
        if path.file_name().is_some_and(|name| {
            name.to_string_lossy()
                .starts_with("org.eclipse.equinox.launcher_")
        }) && path
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

pub(super) fn java(
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<PathBuf, InstallError> {
    let java = environment.program(&["java"], "Java LSP requires Java 21+ (set JAVA_HOME)")?;
    let version = environment.run(Command::new(&java).arg("--version"), stop)?;
    if java_major(&String::from_utf8_lossy(&version)).is_some_and(|version| version >= 21) {
        Ok(java)
    } else {
        Err(InstallError::Missing(
            "Java LSP requires Java 21+ (set JAVA_HOME)",
        ))
    }
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

pub(super) fn find_any(
    names: &[&str],
    paths: impl Iterator<Item = PathBuf>,
    message: &'static str,
) -> Result<PathBuf, InstallError> {
    for directory in paths.filter(|path| path.is_absolute()) {
        for name in names {
            let path = directory.join(executable(name));
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err(InstallError::Missing(message))
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
