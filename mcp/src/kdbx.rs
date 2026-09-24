//! Read-only `KeePass` export support for the `kbdx` MCP tool.
//!
//! JSON and YAML serialize the complete decrypted model exposed by the
//! `keepass` crate. Protected values are therefore plaintext in the returned
//! text. The dotenv representation is deliberately narrower: it contains one
//! variable for every current entry that has a password field.

use keepass::{Database, DatabaseKey};
use std::fs::File;
use std::path::Path;

mod env;

/// A generous ceiling that prevents an untrusted MCP request from making the
/// server parse an arbitrarily large encrypted container in memory.
pub const MAX_DATABASE_BYTES: u64 = 64 * 1024 * 1024;

/// Do not place an unbounded plaintext secret document on the stdio transport.
/// The export is rejected, never truncated, so a successful response is whole.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Text encodings offered by the MCP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Env,
}

/// Failures are intentionally coarse: decryption errors must never echo the
/// supplied password, key material, or decrypted database data.
#[derive(Debug)]
pub enum ExportError {
    Read(std::io::Error),
    InvalidKeyFile,
    Decrypt,
    Serialize,
    DatabaseTooLarge(u64),
    OutputTooLarge(usize),
}

/// Open a `KeePass` database and return one complete text representation.
#[expect(
    clippy::map_err_ignore,
    reason = "a serializer error may quote decrypted data, so only its kind survives"
)]
pub fn export(
    path: &Path,
    password: Option<&str>,
    keyfile: Option<&Path>,
    format: OutputFormat,
) -> Result<String, ExportError> {
    let size = path.metadata().map_err(ExportError::Read)?.len();
    if size > MAX_DATABASE_BYTES {
        return Err(ExportError::DatabaseTooLarge(size));
    }

    let database = open_database(path, password, keyfile)?;
    let text = match format {
        OutputFormat::Json => {
            // Going through `Value` sorts object keys with serde_json's default
            // map, making otherwise HashMap-backed KeePass exports stable.
            let value = serde_json::to_value(&database).map_err(|_| ExportError::Serialize)?;
            serde_json::to_string_pretty(&value).map_err(|_| ExportError::Serialize)?
        }
        OutputFormat::Yaml => {
            let value = serde_json::to_value(&database).map_err(|_| ExportError::Serialize)?;
            serde_norway::to_string(&value).map_err(|_| ExportError::Serialize)?
        }
        OutputFormat::Env => env::export_passwords_as_env(&database),
    };

    if text.len() > MAX_OUTPUT_BYTES {
        return Err(ExportError::OutputTooLarge(text.len()));
    }
    Ok(text)
}

#[expect(
    clippy::map_err_ignore,
    reason = "key-file and decryption errors must never echo key material or vault data"
)]
fn open_database(
    path: &Path,
    password: Option<&str>,
    keyfile: Option<&Path>,
) -> Result<Database, ExportError> {
    let mut key = DatabaseKey::new();
    if let Some(password) = password {
        key = key.with_password(password);
    }
    if let Some(path) = keyfile {
        let mut file = File::open(path).map_err(ExportError::Read)?;
        key = key
            .with_keyfile(&mut file)
            .map_err(|_| ExportError::InvalidKeyFile)?;
    }

    let mut file = File::open(path).map_err(ExportError::Read)?;
    Database::open(&mut file, key).map_err(|_| ExportError::Decrypt)
}

#[cfg(test)]
mod tests;
