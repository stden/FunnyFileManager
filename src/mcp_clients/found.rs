//! Purpose: The `ffm-mcp` entry a config holds, as either format reads it,
//! and how it compares with the entry FFM would write.

use super::{ConfigState, ProjectDir, ServerEntry};

/// The `ffm-mcp` entry a config holds, read out of either format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FoundEntry {
    pub(super) command: Option<String>,
    pub(super) args: Args,
    pub(super) root: Option<String>,
    pub(super) launch: Launch,
    pub(super) switch: Switch,
}

/// Whether an entry passes the server any arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Args {
    /// Absent or `[]`.
    Empty,
    Given,
}

/// Whether the client starts the entry the way FFM writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Launch {
    /// A local program, started in whatever folder the client picks.
    AsWritten,
    /// A `url`, a `cwd`, or a `type` other than `stdio`: not this server, or
    /// not confined where the scope says.
    Elsewhere,
}

/// Whether the client starts the entry at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Switch {
    On,
    /// `"disabled": true` or `enabled = false`, in either format.
    Off,
}

/// The keys that start a server some other way than FFM writes it, and so
/// go when the entry is set: `type` only when it is not `stdio`.
pub(super) const LAUNCH_KEYS: [&str; 3] = ["url", "cwd", "type"];

/// The one `type` an entry may keep.
const STDIO: &str = "stdio";

impl FoundEntry {
    /// Every other key in the entry, and in its `env`, is somebody else's.
    pub(super) fn matches(&self, desired: &ServerEntry) -> bool {
        self.command.as_deref() == Some(desired.command.as_str())
            && self.args == Args::Empty
            && self.launch == Launch::AsWritten
            && self.root.as_deref() == desired.root.as_ref().map(ProjectDir::as_str)
    }

    pub(super) fn state(self, desired: Option<&ServerEntry>) -> ConfigState {
        match desired {
            None => ConfigState::Unverified {
                command: self.command,
            },
            Some(desired) if !self.matches(desired) => ConfigState::DifferentEntry,
            Some(_) if self.switch == Switch::Off => ConfigState::Disabled,
            Some(_) => ConfigState::Registered,
        }
    }
}

impl Launch {
    /// `Elsewhere` when any launch key would go on a write. `text` reads one
    /// key: `None` when it is absent, `Some(None)` when it is not text.
    pub(super) fn read<'a>(text: impl Fn(&str) -> Option<Option<&'a str>>) -> Self {
        let dropped = LAUNCH_KEYS
            .iter()
            .any(|key| text(key).is_some_and(|value| drops_launch_key(key, value)));
        if dropped {
            Self::Elsewhere
        } else {
            Self::AsWritten
        }
    }
}

impl Switch {
    /// `Off` when `disabled` is `true` or `enabled` is `false`; `flag` reads
    /// one key as a boolean.
    pub(super) fn read(flag: impl Fn(&str) -> Option<bool>) -> Self {
        if flag("disabled") == Some(true) || flag("enabled") == Some(false) {
            Self::Off
        } else {
            Self::On
        }
    }
}

/// Whether the entry's launch key `key`, holding `text` (when it is text),
/// goes when the entry is set.
pub(super) fn drops_launch_key(key: &str, text: Option<&str>) -> bool {
    key != "type" || text != Some(STDIO)
}
