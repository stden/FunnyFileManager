//! Purpose: Translate trusted recipes into isolated package-manager operations.

use super::{archive, check_cancel, environment::Environment, launch, InstallError};
use crate::config::{InstallCommand, InstallRecipe};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};

pub(super) fn install(
    recipe: &InstallRecipe,
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    match recipe {
        InstallRecipe::Python { package, .. } => python(package, root, stop, environment),
        InstallRecipe::Npm { packages, .. } => npm(packages, root, stop, environment),
        InstallRecipe::Jdtls { url, sha256 } => {
            launch::java(stop, environment)?; // Diagnose an incompatible runtime before downloading.
            archive::download(url, sha256, root, stop, environment)
        }
        InstallRecipe::Command { steps, .. } => custom(steps, root, stop, environment),
    }
}

fn python(
    package: &str,
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    package_argument(package)?;
    let python = environment.program(
        &["python3", "python"],
        "LSP installation requires Python 3 with venv/pip",
    )?;
    environment.run(
        Command::new(python)
            .args(["-m", "venv"])
            .arg(root.join("venv"))
            .current_dir(root),
        stop,
    )?;
    environment.run(
        Command::new(launch::venv_python(root))
            .args([
                "-m",
                "pip",
                "install",
                "--disable-pip-version-check",
                "--no-input",
                package,
            ])
            .current_dir(root),
        stop,
    )?;
    Ok(())
}

fn npm(
    packages: &[String],
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    if packages.is_empty() {
        return Err(InstallError::Recipe);
    }
    for package in packages {
        package_argument(package)?;
    }
    let node = environment.program(&["node"], "LSP installation requires Node.js and npm")?;
    let npm = npm_cli(&node)?;
    environment.run(
        Command::new(node)
            .arg(npm)
            .args([
                "install",
                "--no-audit",
                "--no-fund",
                "--no-package-lock",
                "--prefix",
            ])
            .arg(root)
            .args(packages)
            .current_dir(root),
        stop,
    )?;
    Ok(())
}

fn npm_cli(node: &Path) -> Result<std::path::PathBuf, InstallError> {
    let parent = node.parent().ok_or(InstallError::Recipe)?;
    let candidates = [
        parent.join("node_modules/npm/bin/npm-cli.js"),
        parent.join("../lib/node_modules/npm/bin/npm-cli.js"),
        parent.join("../share/nodejs/npm/bin/npm-cli.js"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or(InstallError::Missing(
            "LSP installation requires npm beside Node.js",
        ))
}

fn custom(
    steps: &[InstallCommand],
    root: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<(), InstallError> {
    if steps.is_empty() {
        return Err(InstallError::Recipe);
    }
    for step in steps {
        check_cancel(stop)?;
        let expand = |value: &str| value.replace("{install_dir}", &root.to_string_lossy());
        let mut command = Command::new(expand(&step.command));
        command
            .args(step.args.iter().map(|arg| expand(arg)))
            .current_dir(root);
        for (key, value) in &step.env {
            command.env(key, expand(value));
        }
        environment.run(&mut command, stop)?;
    }
    Ok(())
}

fn package_argument(package: &str) -> Result<(), InstallError> {
    if package.is_empty() || package.starts_with('-') || package.contains(['\0', '\r', '\n']) {
        Err(InstallError::Recipe)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
