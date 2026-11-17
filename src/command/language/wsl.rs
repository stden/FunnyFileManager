//! Purpose: The WSL manager's key language: the commands its keys translate into.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslCommand {
    Up,
    Down,
    First,
    Last,
    Open,
    Refresh,
    Update,
    UpdatePackages,
    Close,
    /// The answer to a key nothing is bound to: the dialog consumes it.
    Ignore,
}
