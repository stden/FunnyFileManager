//! Purpose: Patch workflows for Things-managed RHEL and Windows virtual machines.

mod plan;
mod rpc;

use super::Machine;

pub use plan::build_plan;
#[cfg(test)]
pub use rpc::{build_start_payload, execute_plan_with};
pub use rpc::{execute_plan, poll_group, poll_group_with, start_wave, start_wave_with};

/// Operating system family supported by automated Things patching workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatchKind {
    Rhel,
    Windows,
}

impl PatchKind {
    /// GraphQL workflow `typeNameUri` for this platform.
    #[must_use]
    pub const fn workflow_uri(self) -> &'static str {
        match self {
            Self::Rhel => "patch_vm_rhel",
            Self::Windows => "patch_vm_windows",
        }
    }

    /// Short human-readable label for dialogs and CLI output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rhel => "RHEL",
            Self::Windows => "Windows",
        }
    }

    /// Classify a catalog machine by its operating system name or tags.
    #[must_use]
    pub fn classify(machine: &Machine) -> Option<Self> {
        let os_name = machine.os_info.as_ref().map_or("", |os| os.name.as_str());
        if contains_ascii_ci(os_name, "windows")
            || machine
                .tags
                .iter()
                .any(|tag| contains_ascii_ci(tag, "windows"))
        {
            return Some(Self::Windows);
        }
        if is_rhel_text(os_name) || machine.tags.iter().any(|tag| is_rhel_text(tag)) {
            return Some(Self::Rhel);
        }
        None
    }
}

/// Scope filter for server patch runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PatchScope {
    #[default]
    All,
    Rhel,
    Windows,
}

impl PatchScope {
    /// Parse a CLI scope value (`all`, `rhel`, `windows`).
    #[must_use]
    pub fn parse_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "all" => Some(Self::All),
            "rhel" | "linux" => Some(Self::Rhel),
            "windows" | "win" => Some(Self::Windows),
            _ => None,
        }
    }

    /// Whether this scope includes `kind`.
    #[must_use]
    pub const fn allows(self, kind: PatchKind) -> bool {
        matches!(
            (self, kind),
            (Self::All, _) | (Self::Rhel, PatchKind::Rhel) | (Self::Windows, PatchKind::Windows)
        )
    }
}

/// One VM targeted for a patch workflow wave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTarget {
    pub id: String,
    pub name: String,
    pub kind: PatchKind,
}

/// A batch of same-OS VMs started together in one workflow execution group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchWave {
    pub index: usize,
    pub kind: PatchKind,
    pub targets: Vec<PatchTarget>,
}

/// Ordered patch waves built from catalog machines, preserving HA separation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPlan {
    pub waves: Vec<PatchWave>,
    pub skipped: Vec<String>,
    pub reboot: bool,
    pub comment: String,
}

impl PatchPlan {
    /// Total VM targets scheduled across all waves.
    #[must_use]
    pub fn target_count(&self) -> usize {
        self.waves.iter().map(|wave| wave.targets.len()).sum()
    }
}

/// Status of one child workflow execution returned by Things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionInfo {
    pub id: String,
    pub status: String,
}

/// Result of starting or polling one workflow execution group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupResult {
    pub id: String,
    pub status: String,
    pub executions: Vec<ExecutionInfo>,
}

/// Aggregated result after starting all waves in a [`PatchPlan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReport {
    pub groups: Vec<GroupResult>,
}

impl PatchReport {
    /// Short summary count of started groups and executions.
    #[must_use]
    pub fn summary(&self) -> String {
        let executions: usize = self.groups.iter().map(|group| group.executions.len()).sum();
        format!("{} wave(s), {executions} execution(s)", self.groups.len())
    }
}

/// Whether a Things workflow execution or group status is terminal.
#[must_use]
pub fn is_terminal_status(status: &str) -> bool {
    let upper = status.trim().to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "FINISHED" | "FAILED" | "REJECTED" | "CANCELED" | "CANCELLED"
    )
}

/// Whether a Things workflow status completed successfully.
#[must_use]
pub fn is_success_status(status: &str) -> bool {
    status.trim().eq_ignore_ascii_case("FINISHED")
}

fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn is_rhel_text(text: &str) -> bool {
    ["red hat", "rhel", "almalinux", "rocky", "centos", "linux"]
        .iter()
        .any(|token| contains_ascii_ci(text, token))
}
