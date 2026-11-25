//! Purpose: Jira settings read from `ffm.yml`, defaults, and env variable names.

use std::path::PathBuf;

/// Default JQL matching `sync_kn_tickets.py` open-issue query.
pub const DEFAULT_JQL: &str =
    "assignee = currentUser() AND status not in (Closed, Done, Resolved) ORDER BY updated DESC";

/// Default JQL matching `export_my_jira.py` snapshot query.
pub const DEFAULT_SNAPSHOT_JQL: &str = "assignee = currentUser() ORDER BY updated DESC";

/// Environment variables checked in order for the Jira personal access token.
pub const TOKEN_VARIABLES: &[&str] = &[
    "JIRA_PERSONAL_TOKEN",
    "ATLASSIAN_JIRA_PERSONAL_TOKEN",
    "JIRA_API_TOKEN",
    "JIRA_TOKEN",
    "JIRA_PAT",
];

/// Whether background Jira requests may reach the network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JiraAccess {
    #[default]
    Offline,
    Network,
}

/// Optional `jira:` section inside user `ffm.yml`. Secrets stay in env vars only.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct JiraSettings {
    pub url: Option<String>,
    pub jql: Option<String>,
    pub export_dir: Option<PathBuf>,
    pub tls_verify: Option<bool>,
    pub ca_cert: Option<PathBuf>,
    pub page_size: Option<u16>,
}
