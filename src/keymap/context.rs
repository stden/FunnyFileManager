//! Purpose: `Context`, the fourteen screens whose keys `ffm.yml` holds, each
//! with the name of its section in the file.

use std::fmt;

/// A screen with a key table of its own: a section of `ffm.yml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Context {
    Panels,
    TreePanel,
    NetboxPanel,
    CommandLine,
    CommandLineSearch,
    CommandLineCompletion,
    Viewer,
    Editor,
    TaskList,
    TaskListSearch,
    WslManager,
    LanguageServers,
    PathEditor,
    Vault,
}

impl Context {
    /// Every context, in the order the default file writes them.
    pub const ALL: [Self; 14] = [
        Self::Panels,
        Self::TreePanel,
        Self::NetboxPanel,
        Self::CommandLine,
        Self::CommandLineSearch,
        Self::CommandLineCompletion,
        Self::Viewer,
        Self::Editor,
        Self::TaskList,
        Self::TaskListSearch,
        Self::WslManager,
        Self::LanguageServers,
        Self::PathEditor,
        Self::Vault,
    ];

    /// The section's key in `ffm.yml`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Panels => "panels",
            Self::TreePanel => "tree_panel",
            Self::NetboxPanel => "netbox_panel",
            Self::CommandLine => "command_line",
            Self::CommandLineSearch => "command_line_search",
            Self::CommandLineCompletion => "command_line_completion",
            Self::Viewer => "viewer",
            Self::Editor => "editor",
            Self::TaskList => "task_list",
            Self::TaskListSearch => "task_list_search",
            Self::WslManager => "wsl_manager",
            Self::LanguageServers => "language_servers",
            Self::PathEditor => "path_editor",
            Self::Vault => "vault",
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.name())
    }
}
