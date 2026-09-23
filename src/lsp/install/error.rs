//! Purpose: Fixed, non-sensitive installation failures suitable for the editor status line.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallError {
    Io(std::io::ErrorKind),
    Missing(&'static str),
    Recipe,
    Command,
    Timeout,
    Cancelled,
    Integrity,
    Archive,
    Worker,
}

impl InstallError {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Io(_) => "LSP installation: storage or process error (Ctrl+Alt+R retries)",
            Self::Missing(message) => message,
            Self::Recipe => "LSP installation: invalid recipe or missing server executable",
            Self::Command => {
                "LSP installation failed: check network/package manager; Ctrl+Alt+R retries"
            }
            Self::Timeout => "LSP installation timed out (Ctrl+Alt+R retries)",
            Self::Cancelled => "LSP installation cancelled",
            Self::Integrity => "LSP download checksum mismatch",
            Self::Archive => "LSP installation: unsafe or invalid archive",
            Self::Worker => "LSP installer stopped unexpectedly (Ctrl+Alt+R retries)",
        }
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
