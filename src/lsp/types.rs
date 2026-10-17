//! Purpose: Typed document identities, editor requests, and server-visible state.

use super::positions::Position;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentId(u64);

impl Default for DocumentId {
    fn default() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Revision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub id: DocumentId,
    pub revision: Revision,
}

/// Whether a completion request was invoked explicitly (`Ctrl+Space`) or
/// automatically after typing an identifier/operator character.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTrigger {
    #[default]
    Manual,
    AutoWhileTyping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Completion(CompletionTrigger),
    Hover,
    Definition,
}

impl Feature {
    pub const fn method(self) -> &'static str {
        match self {
            Self::Completion(_) => "textDocument/completion",
            Self::Hover => "textDocument/hover",
            Self::Definition => "textDocument/definition",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ServerState {
    #[default]
    Stopped,
    Unavailable,
    Untrusted,
    Starting,
    Installing,
    InstallationFailed,
    Initializing,
    Ready,
    Failed,
}

impl ServerState {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Stopped => "LSP off",
            Self::Unavailable => "LSP unavailable: check command/PATH",
            Self::Untrusted => "LSP needs workspace trust (Ctrl+Alt+R)",
            Self::Starting => "LSP starting",
            Self::Installing => "LSP installing in background (close editor to cancel)",
            Self::InstallationFailed => "LSP installation failed (Ctrl+Alt+R retries)",
            Self::Initializing => "LSP initializing",
            Self::Ready => "LSP ready",
            Self::Failed => "LSP failed (Ctrl+Alt+R to restart)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub stamp: Stamp,
    pub path: PathBuf,
    pub text: String,
    pub saved: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    pub stamp: Stamp,
    pub position: Position,
    pub feature: Feature,
}
