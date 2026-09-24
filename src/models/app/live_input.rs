//! Purpose: What only the production entry point switches on: the pause after
//! a program ends, and the real machine the MCP and LSP dialogs look at.

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
/// A test's or a replayed recording's `App` keeps all three off: nobody is at
/// its terminal to end a pause, replaying a session must never edit a real MCP
/// client's config or start a real client, and no test or replay may read the
/// real machine's `PATH` or LSP cache.
#[derive(Clone, Default)]
pub struct LiveInput {
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
}
