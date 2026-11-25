//! Modal data for F9 prompts and selectable results.

use std::path::PathBuf;

use crate::{
    lineedit::LineEditor,
    menu::{MenuCommand, SettingsPage},
};

/// What the menu's one-line prompt asks for, and so what its answer does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPrompt {
    /// The active panel's file mask (Commands → File panel filter).
    Filter,
    /// A folder mask to search for (Commands → Find folder).
    FindFolder,
    /// One Options page's value.
    Settings(SettingsPage),
}

#[derive(Debug, Clone)]
pub enum MenuDialog {
    Prompt {
        purpose: MenuPrompt,
        title: String,
        caption: String,
        input: LineEditor,
    },
    Choices {
        title: String,
        rows: Vec<MenuChoice>,
        selected: usize,
    },
}

#[derive(Debug, Clone)]
pub struct MenuChoice {
    pub label: String,
    pub target: ChoiceTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeOperation {
    Trim,
    Defragment,
}

impl VolumeOperation {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Trim => "TRIM",
            Self::Defragment => "Defragment",
        }
    }
}

/// The confirmed mount point and the Windows volume identity behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeTarget {
    pub(crate) mount: PathBuf,
    pub(crate) id: std::ffi::OsString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSyncTarget {
    pub(crate) root: PathBuf,
    pub(crate) branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceCommand {
    Volume(VolumeOperation, VolumeTarget),
    SyncBranch(GitSyncTarget),
    ConnectNetworkDrives(Vec<super::NetworkDrive>),
}

impl MaintenanceCommand {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Volume(operation, target) => {
                format!(
                    "{} {}",
                    operation.label(),
                    crate::path_display::label(&target.mount)
                )
            }
            Self::SyncBranch(target) => format!(
                "Update Git branch {} in {}",
                target.branch,
                crate::path_display::label(&target.root)
            ),
            Self::ConnectNetworkDrives(drives) => {
                format!("Connect {} network drive(s)", drives.len())
            }
        }
    }
}

/// A choice reads as its label, which is all a menu draws of it.
impl AsRef<str> for MenuChoice {
    fn as_ref(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone)]
pub enum ChoiceTarget {
    RunMaintenance(MaintenanceCommand),
    UpdatePackages(crate::packages::UpdatePlan),
    Directory(PathBuf),
    File(PathBuf),
    /// A file, opened in the editor with the cursor on its 1-based `line`.
    FileLine {
        path: PathBuf,
        line: usize,
    },
    Command(String),
    Screen(super::ViewMode),
    Menu(MenuCommand),
    Close,
}
