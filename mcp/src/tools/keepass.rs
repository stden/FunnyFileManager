//! Purpose: The read-only `kbdx` tool: decrypt a `KeePass` vault inside the
//! root and return it as JSON, YAML or dotenv text.

use crate::{kdbx, FfmServer};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_router, ErrorData as McpError,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Output representation for the `kbdx` tool.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KdbxFormat {
    /// Pretty, deterministic JSON containing the complete decrypted model.
    #[default]
    Json,
    /// YAML containing the same complete decrypted model as JSON.
    Yaml,
    /// Dotenv variables for current entry passwords only.
    Env,
}

impl From<KdbxFormat> for kdbx::OutputFormat {
    fn from(value: KdbxFormat) -> Self {
        match value {
            KdbxFormat::Json => Self::Json,
            KdbxFormat::Yaml => Self::Yaml,
            KdbxFormat::Env => Self::Env,
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct KdbxParams {
    /// Path to an existing `.kdbx` database; common `.kbdx` transposition accepted.
    pub path: String,
    /// Master password. Omit only for a key-file-only database. Never logged.
    #[serde(default)]
    pub password: Option<String>,
    /// Optional `KeePass` key file; it must also be inside the allowed root.
    #[serde(default)]
    pub keyfile: Option<String>,
    /// Output format: `json` (default), `yaml`, or password-only `env`.
    #[serde(default)]
    pub format: KdbxFormat,
}

#[tool_router(router = keepass_tools, vis = "pub(super)")]
impl FfmServer {
    /// Decrypt a `KeePass` database and return its contents as JSON, YAML, or dotenv text.
    /// JSON/YAML contain the complete model, including plaintext protected values;
    /// dotenv contains current password fields only. Nothing is written to disk.
    #[tool(
        description = "Decrypt a .kdbx/.kbdx KeePass database and return it as JSON (default), YAML, or password-only dotenv text. The response contains plaintext secrets. The database and optional key file must be inside the allowed root; no output file is written.",
        annotations(
            title = "Export KeePass database",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub(crate) fn kbdx(
        &self,
        Parameters(params): Parameters<KdbxParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_kdbx_allowed()?;
        if params.password.is_none() && params.keyfile.is_none() {
            return Err(McpError::invalid_params(
                "provide a password, a key file, or both",
                None,
            ));
        }

        let path = self.safe_file(&params.path)?;
        validate_vault_extension(&path)?;

        let keyfile = params
            .keyfile
            .as_deref()
            .map(|raw| self.safe_path(raw))
            .transpose()?;
        if keyfile.as_ref().is_some_and(|path| !path.is_file()) {
            return Err(McpError::invalid_params(
                "KeePass key file path is not a file",
                None,
            ));
        }

        let output = kdbx::export(
            &path,
            params.password.as_deref(),
            keyfile.as_deref(),
            params.format.into(),
        )
        .map_err(export_error)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
    }
}

fn validate_vault_extension(path: &Path) -> Result<(), McpError> {
    let supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("kdbx") || extension.eq_ignore_ascii_case("kbdx")
        });
    if !supported_extension {
        return Err(McpError::invalid_params(
            "KeePass database path must end in .kdbx or .kbdx",
            None,
        ));
    }

    Ok(())
}

fn export_error(error: kdbx::ExportError) -> McpError {
    match error {
        kdbx::ExportError::Read(source) => {
            McpError::internal_error(format!("failed to read KeePass data: {source}"), None)
        }
        kdbx::ExportError::InvalidKeyFile => {
            McpError::invalid_params("invalid KeePass key file", None)
        }
        kdbx::ExportError::Decrypt => McpError::invalid_params(
            "could not decrypt KeePass database (wrong credentials or invalid database)",
            None,
        ),
        kdbx::ExportError::Serialize => {
            McpError::internal_error("could not serialize KeePass database", None)
        }
        kdbx::ExportError::DatabaseTooLarge(bytes) => McpError::invalid_params(
            format!(
                "KeePass database is {bytes} bytes; maximum is {} bytes",
                kdbx::MAX_DATABASE_BYTES
            ),
            None,
        ),
        kdbx::ExportError::OutputTooLarge(bytes) => McpError::invalid_params(
            format!(
                "KeePass text export is {bytes} bytes; maximum is {} bytes",
                kdbx::MAX_OUTPUT_BYTES
            ),
            None,
        ),
    }
}
