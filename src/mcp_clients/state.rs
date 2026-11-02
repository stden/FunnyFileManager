//! Purpose: What a client's config says about `ffm-mcp`, and the words the
//! dialog and the `:mcp` screen both use for it.

/// A config's state for the scope being looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigState {
    /// The file is not there.
    NoConfig,
    /// The file parses and has no `ffm-mcp` entry.
    NotConfigured,
    /// The entry runs this server, with no arguments, confined as the scope
    /// asks.
    Registered,
    /// Registered, but switched off (`"disabled": true`, `enabled = false`):
    /// the client's owner said so, and registering leaves it off.
    Disabled,
    /// An entry runs another command, passes arguments, has another root, or
    /// is started some other way (`url`, `cwd`, a `type` other than stdio).
    DifferentEntry,
    /// An entry is there, but no server was found to compare it with.
    /// `command` is what it runs, when it names one.
    Unverified { command: Option<String> },
    /// An entry with nothing to compare it with, whose command is an absolute
    /// path to nothing: it cannot start.
    ProgramMissing,
    /// Nothing will be written here, and why.
    Blocked(Blocker),
}

impl ConfigState {
    /// The row's state as the dialog and the `:mcp` screen show it.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::NoConfig => "— no config".to_owned(),
            Self::NotConfigured => "⊘ not configured".to_owned(),
            Self::DifferentEntry => "≠ different entry".to_owned(),
            Self::Registered => "✓ registered".to_owned(),
            Self::Disabled => "✓ registered (disabled)".to_owned(),
            Self::Unverified { .. } => "• entry present, server not found".to_owned(),
            Self::ProgramMissing => "≠ entry runs a missing program".to_owned(),
            Self::Blocked(blocker) => format!("✗ {}", blocker.label()),
        }
    }

    /// Whether registering would change the file.
    #[must_use]
    pub const fn is_actionable(&self) -> bool {
        matches!(
            self,
            Self::NoConfig | Self::NotConfigured | Self::DifferentEntry
        )
    }
}

/// Why a config will not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocker {
    /// The parser's message, with the line and column.
    Unparsable(String),
    /// The key that is not an object (a table, in TOML).
    UnexpectedLayout(&'static str),
    /// Why the file could not be read.
    Unreadable(String),
    /// Claude Code's user config is written by the `claude` CLI only, and it
    /// is not on PATH.
    NeedsClaudeCli,
    /// The variable that should name the client's home folder does not.
    HomeVarNotFolder(&'static str),
    NoHomeFolder,
    /// The folder's project-level config is the client's user-level one — the
    /// active panel is the home folder, or the client's home is inside the
    /// folder — so Current folder would confine every project to it.
    SameAsUserConfig,
}

impl Blocker {
    /// The short form, for a dialog row.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Unparsable(_) => "config does not parse".to_owned(),
            Self::UnexpectedLayout(_) => "unexpected config layout".to_owned(),
            Self::Unreadable(_) => "config unreadable".to_owned(),
            Self::NeedsClaudeCli => "needs the claude CLI".to_owned(),
            Self::HomeVarNotFolder(var) => format!("${var} is not a folder"),
            Self::NoHomeFolder => "no home folder".to_owned(),
            Self::SameAsUserConfig => "same file as All projects".to_owned(),
        }
    }

    /// The long form, for the report: what is wrong with `config` and what to
    /// do about it. `config` is the file when its path is known.
    #[must_use]
    pub fn explain(&self, config: &str) -> String {
        match self {
            Self::Unparsable(message) => format!(
                "{config} does not parse ({message}); left untouched. \
                 Fix or remove it, then register again."
            ),
            Self::UnexpectedLayout(key) => {
                format!("{config}: {key} is not an object; left untouched")
            }
            Self::Unreadable(message) => format!("cannot read {config}: {message}"),
            Self::NeedsClaudeCli => "the claude CLI is not on PATH".to_owned(),
            Self::HomeVarNotFolder(var) => format!("${var} does not name a folder"),
            Self::NoHomeFolder => "there is no home folder to register in".to_owned(),
            Self::SameAsUserConfig => format!(
                "{config} is also the All projects config; left untouched. \
                 Register at All projects, or from a project's own folder."
            ),
        }
    }
}
