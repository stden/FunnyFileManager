//! Purpose: Jira background job requests, replies, and typed failures.

use std::fmt;
use std::path::PathBuf;

use super::{JiraIssue, JiraUser};

/// What a background Jira worker is currently doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JiraBusy {
    Fetching,
    Exporting,
}

impl JiraBusy {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fetching => "Loading Jira issues...",
            Self::Exporting => "Exporting Jira issues...",
        }
    }
}

/// Fetched Jira issues together with server identity and effective query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JiraListing {
    pub user: JiraUser,
    pub issues: Vec<JiraIssue>,
    pub total: usize,
    pub jql: String,
    pub base_url: String,
}

/// Result summary for a per-ticket or snapshot export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    pub summary: String,
    pub output_dir: PathBuf,
    pub written: usize,
    pub skipped: usize,
}

/// Background request handled by the Jira worker.
#[derive(Debug, Clone)]
pub enum JiraRequest {
    Fetch {
        jql: String,
        page_size: u16,
    },
    ExportTickets {
        issues: Vec<JiraIssue>,
        user: Option<JiraUser>,
        base_url: String,
        output_dir: PathBuf,
        force: bool,
    },
    ExportKey {
        key: String,
        output_dir: PathBuf,
        force: bool,
    },
    ExportSnapshot {
        jql: String,
        page_size: u16,
        output_dir: PathBuf,
    },
}

impl JiraRequest {
    #[must_use]
    pub const fn busy(&self) -> JiraBusy {
        match self {
            Self::Fetch { .. } => JiraBusy::Fetching,
            Self::ExportTickets { .. } | Self::ExportKey { .. } | Self::ExportSnapshot { .. } => {
                JiraBusy::Exporting
            }
        }
    }
}

/// Background reply delivered back to the UI state.
#[derive(Debug, Clone)]
pub enum JiraReply {
    Fetched(Result<JiraListing, JiraError>),
    Exported(Result<ExportReport, JiraError>),
}

impl JiraReply {
    /// Wrap `error` into the matching reply variant for `request`.
    #[must_use]
    pub const fn fail(request: &JiraRequest, error: JiraError) -> Self {
        match request {
            JiraRequest::Fetch { .. } => Self::Fetched(Err(error)),
            JiraRequest::ExportTickets { .. }
            | JiraRequest::ExportKey { .. }
            | JiraRequest::ExportSnapshot { .. } => Self::Exported(Err(error)),
        }
    }
}

/// Typed Jira adapter errors; tokens and private URLs are never formatted into these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JiraError {
    Disabled,
    Config(String),
    Network(String),
    Http(u16, String),
    InvalidResponse(String),
    Io(String),
    Worker,
}

impl fmt::Display for JiraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("Unavailable in replay or isolated sessions"),
            Self::Config(msg) | Self::Network(msg) | Self::InvalidResponse(msg) => f.write_str(msg),
            Self::Http(code, msg) => write!(f, "Jira HTTP {code}: {msg}"),
            Self::Io(msg) => write!(f, "Jira export failed: {msg}"),
            Self::Worker => f.write_str("Jira worker stopped unexpectedly"),
        }
    }
}

#[cfg(test)]
mod tests;
