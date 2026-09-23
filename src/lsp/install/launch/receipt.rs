//! Purpose: Resolve cached launch receipts against the current runtime and canonical workspace.

use super::{
    fs, runnable, storage, Environment, InstallError, InstallRecipe, Launch, Path, PathBuf,
    ServerSettings,
};

impl Launch {
    pub fn settings(
        &self,
        settings: &ServerSettings,
        root: &Path,
        workspace: &Path,
        environment: &dyn Environment,
    ) -> Result<ServerSettings, InstallError> {
        let prepared = self.preview(settings, root, workspace, environment)?;
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

    pub(in crate::lsp::install) fn preview(
        &self,
        settings: &ServerSettings,
        root: &Path,
        workspace: &Path,
        environment: &dyn Environment,
    ) -> Result<ServerSettings, InstallError> {
        let data = workspace_data(root, workspace);
        let mut prepared = settings.clone();
        prepared.command = self.command(settings.install.as_ref(), root, &data, environment)?;
        if !runnable(Path::new(&prepared.command)) {
            return Err(InstallError::Recipe);
        }
        prepared.args = self
            .args
            .iter()
            .chain(&settings.args)
            .map(|arg| expand(arg, root, &data))
            .collect();
        prepared.install = None;
        Ok(prepared)
    }

    fn command(
        &self,
        recipe: Option<&InstallRecipe>,
        root: &Path,
        data: &Path,
        environment: &dyn Environment,
    ) -> Result<String, InstallError> {
        let runtime = match recipe {
            Some(InstallRecipe::Npm { .. }) => Some(("node", "LSP installation requires Node.js")),
            Some(InstallRecipe::Jdtls { .. }) => {
                Some(("java", "Java LSP requires Java 21+ (set JAVA_HOME)"))
            }
            _ => None, // Python and custom recipes own their cached executable.
        };
        runtime.map_or_else(
            || Ok(expand(&self.command, root, data)),
            |(name, missing)| {
                environment
                    .program(&[name], missing)
                    .map(|path| path.to_string_lossy().into_owned())
            },
        )
    }
}

fn workspace_data(root: &Path, workspace: &Path) -> PathBuf {
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    root.join("workspaces")
        .join(storage::digest(workspace.as_os_str().as_encoded_bytes()))
}

fn expand(value: &str, root: &Path, data: &Path) -> String {
    value
        .replace("{install_dir}", &root.to_string_lossy())
        .replace("{workspace_data}", &data.to_string_lossy())
}
