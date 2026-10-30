//! Purpose: Fixed, non-sensitive installation failures suitable for the editor status line.

use std::io::ErrorKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallError {
    Io(ErrorKind),
    /// The LSP cache folder could not be made or written.
    Cache(ErrorKind),
    Missing(&'static str),
    Recipe,
    Command,
    /// A program an install step ran exited with an error.
    Failed(Step),
    Timeout,
    Cancelled,
    Integrity,
    Archive,
    Worker,
}

/// The program an install step runs, named when it exits with an error so
/// the status says what to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `python3 -m venv`, which fails where `python3-venv` (ensurepip) is not installed.
    Venv,
    /// `pip install`.
    Pip,
    /// `npm install`.
    Npm,
    /// `curl`'s download of the server.
    Download,
    /// A step of a custom recipe.
    Custom,
}

impl Step {
    const fn label(self) -> &'static str {
        match self {
            Self::Venv => {
                "LSP installation failed: python3 -m venv failed; install python3-venv (Ctrl+Alt+R retries)"
            }
            Self::Pip => "LSP installation failed: pip install failed; check the network (Ctrl+Alt+R retries)",
            Self::Npm => "LSP installation failed: npm install failed; check the network (Ctrl+Alt+R retries)",
            Self::Download => {
                "LSP installation failed: the download failed; check the network (Ctrl+Alt+R retries)"
            }
            Self::Custom => {
                "LSP installation failed: an install command exited with an error (Ctrl+Alt+R retries)"
            }
        }
    }
}

impl InstallError {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Io(_) => "LSP installation: storage or process error (Ctrl+Alt+R retries)",
            Self::Cache(kind) => cache_label(kind),
            Self::Missing(message) => message,
            Self::Recipe => "LSP installation: invalid recipe or missing server executable",
            Self::Command => {
                "LSP installation failed: check network/package manager; Ctrl+Alt+R retries"
            }
            Self::Failed(step) => step.label(),
            Self::Timeout => "LSP installation timed out (Ctrl+Alt+R retries)",
            Self::Cancelled => "LSP installation cancelled",
            Self::Integrity => "LSP download checksum mismatch",
            Self::Archive => "LSP installation: unsafe or invalid archive",
            Self::Worker => "LSP installer stopped unexpectedly (Ctrl+Alt+R retries)",
        }
    }

    /// This error as `step`'s: a program that exited with an error is named
    /// by the step that ran it; every other error stays as it is.
    #[must_use]
    pub const fn during(self, step: Step) -> Self {
        match self {
            Self::Command => Self::Failed(step),
            other => other,
        }
    }

    /// An error of the LSP cache folder itself.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the shape `map_err` hands over: `.map_err(InstallError::cache)`"
    )]
    #[must_use]
    pub fn cache(error: std::io::Error) -> Self {
        Self::Cache(error.kind())
    }
}

/// Why the LSP cache folder could not be used, as the status says it.
const fn cache_label(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::PermissionDenied => {
            "LSP installation: the LSP cache folder is not writable (Ctrl+Alt+R retries)"
        }
        ErrorKind::ReadOnlyFilesystem => {
            "LSP installation: the LSP cache folder is on a read-only file system (Ctrl+Alt+R retries)"
        }
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => {
            "LSP installation: no space left for the LSP cache (Ctrl+Alt+R retries)"
        }
        _ => "LSP installation: cannot use the LSP cache folder (Ctrl+Alt+R retries)",
    }
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

impl std::error::Error for InstallError {}

impl From<std::io::Error> for InstallError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.kind())
    }
}

#[cfg(test)]
mod tests;
