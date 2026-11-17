//! Purpose: Whether the event loop keeps running or something asked to leave.

/// Whether the event loop should keep running.
///
/// One representation, replacing `should_quit: bool` *beside* a `Flow`
/// enum returned by the key dispatcher. The loop condition read the flag while
/// the dispatch site matched the enum, so "are we leaving" had two answers
/// that had to be kept in step by hand — and the overlay layer could only
/// reach one of them, which is why the flag existed at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Running,
    /// Something asked to leave: F10, Ctrl+C, or the `quit` / `exit` command.
    Quitting,
}

impl Lifecycle {
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}
