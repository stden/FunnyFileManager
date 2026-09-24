//! Purpose: Resolve installed server entry points and workspace-specific Java launch arguments.

use super::{environment::Environment, storage, InstallError};
use crate::config::{InstallRecipe, ServerSettings};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

#[derive(Serialize, Deserialize)]
pub(super) struct Launch {
    command: String,
    args: Vec<String>,
    configuration: Option<String>,
}

mod java;
mod programs;
mod receipt;

pub(super) use java::java;
use java::jdtls;
#[cfg(test)]
use java::{java_launcher, java_major};
pub use programs::{executable, launchable_name};
pub(super) use programs::{find_all, runnable, venv_python};

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
    let relative = path
        .strip_prefix(root)
        .map_err(crate::lsp::logged(InstallError::Recipe))?;
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
            .map_err(crate::lsp::logged(InstallError::Recipe))?;
    // `bin` is either the one script's path or a map from command to path.
    let bin = manifest.get("bin").ok_or(InstallError::Recipe)?;
    let bin = bin
        .as_str()
        .or_else(|| bin.get(binary).and_then(serde_json::Value::as_str))
        .ok_or(InstallError::Recipe)?;
    let script = storage::relative(&package, bin)?;
    installed(&script)?;
    Ok(Launch {
        command: node.to_string_lossy().into_owned(),
        args: vec![template(root, &script)?],
        configuration: None,
    })
}

#[cfg(test)]
mod tests;
