//! Purpose: What only the production entry point switches on: the pause after
//! a program ends, the real machine the MCP and LSP dialogs look at, and
//! asking Windows for administrator rights.

/// What happens on the terminal after a program started from a panel ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgramPause {
    /// The panels return at once.
    #[default]
    None,
    /// The output stays on screen until a key is pressed.
    UntilKey,
}

/// What the production entry point switches on for live input only.
///
/// A test's or a replayed recording's `App` keeps all of it off: nobody is at
/// its terminal to end a pause, replaying a session must never edit a real MCP
/// client's config or start a real client, no test or replay may read the
/// real machine's `PATH` or LSP cache, and none may put a UAC prompt on the
/// screen — a suite that copies into a protected folder once raised one per
/// test.
#[derive(Clone, Default)]
pub struct LiveInput {
    /// Only live input may read provider API credentials and usage reports.
    pub(crate) usage: crate::models::token_usage::UsageAccess,
    /// Whether a finished program's output waits for a key before the panels
    /// return.
    pub program_pause: ProgramPause,
    /// The machine the MCP installer and the `:mcp` screen look at. Inert by
    /// default: no home folder, no PATH, no program runner.
    pub mcp_host: crate::mcp_clients::Host,
    /// What the Language servers dialog asks of this machine: the LSP install
    /// cache, `PATH` and `JAVA_HOME`, and the program names it offers. Inert by
    /// default: it finds nothing.
    pub(crate) lsp_probe: crate::lsp::Probe,
    /// Whether a refused copy, move or delete may be retried as an
    /// administrator, which asks Windows (UAC). Off by default.
    pub elevation: bool,
}
