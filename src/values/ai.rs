//! Purpose: Plain AI assistant types shared by `ai`, `models`, `ops`, and `ui`.

mod config;
mod message;
mod request;

pub use config::{AiAccess, AiProvider, AiSettings, ProviderSettings};
pub use message::{ChatMessage, Role, WorkspaceContext};
pub use request::{AiError, AiRequest, AiResponse};
