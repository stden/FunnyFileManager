//! Purpose: Who the prompt says you are — the login name, the machine, and
//! whether this process runs as root — read from the environment.

/// Whether FFM runs as root.
///
/// It decides whether this shell can overwrite anything, which is what `#`
/// means, and whether `[` in the task list lowers a priority at once or asks
/// first, since only root can raise it again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Privilege {
    #[default]
    User,
    Root,
}

impl Privilege {
    /// The privilege of the process actually running.
    #[must_use]
    pub fn current() -> Self {
        if crate::platform::is_root() {
            Self::Root
        } else {
            Self::User
        }
    }
}

/// Who the prompt says you are, and on which machine.
///
/// Passed in rather than read inside [`segments`](super::segments) so the
/// prompt can be tested against a known identity. Reading the environment
/// *inside* the assembly made every one of these values untestable: mutation
/// testing showed `user`, `host` and `is_root` could each return anything at
/// all — including the wrong sigil, which is the one that matters — without a
/// single test noticing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub user: Option<String>,
    pub host: Option<String>,
    pub privilege: Privilege,
}

impl Identity {
    /// Read the identity of the process actually running.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            user: user(),
            host: host(),
            privilege: Privilege::current(),
        }
    }
}

/// Variables that may carry the login name, in the order they are trusted.
const USER_VARS: [&str; 3] = ["USER", "LOGNAME", "USERNAME"];

/// The login name, from the environment.
fn user() -> Option<String> {
    let values: Vec<Option<String>> = USER_VARS
        .iter()
        .map(|name| std::env::var(name).ok())
        .collect();
    pick_user(&values)
}

/// The first candidate that is actually a name.
///
/// Split from the reading so it can be tested: an empty variable is worse than
/// an unset one, because it renders as a bare `@host` — and a function that
/// reads the environment can be given no input at all by a test, which is how
/// this rule went unchecked until mutation testing pointed at it.
pub(super) fn pick_user(candidates: &[Option<String>]) -> Option<String> {
    candidates
        .iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
}

/// The machine's name.
///
/// `/etc/hostname` before `$HOSTNAME`: the variable is not exported by every
/// shell, so on many systems it is simply absent.
fn host() -> Option<String> {
    let file = std::fs::read_to_string("/etc/hostname").ok();
    let variable = std::env::var("HOSTNAME").ok();
    pick_host(file.as_deref(), variable.as_deref())
}

/// The machine's name, from the file if it says one and the variable if not.
///
/// The file wins because it is what the system believes; the variable is a
/// shell's copy of it and is often stale or unset. Both are trimmed, and an
/// empty result from either is treated as absent rather than shown.
pub(super) fn pick_host(file: Option<&str>, variable: Option<&str>) -> Option<String> {
    [file, variable]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}
