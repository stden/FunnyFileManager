//! Purpose: Editor-independent LSP framing, sessions, documents, and server lifecycle.

mod client;
mod document;
pub mod features;
mod install;
pub use install::inventory;
pub mod manager;
pub mod positions;
mod process;
mod rpc;
mod transport;
pub mod types;
pub mod uri;
mod worker;

#[cfg(test)]
mod tests;
