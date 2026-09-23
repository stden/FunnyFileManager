//! Purpose: Editor-independent LSP framing, sessions, documents, and server lifecycle.

mod client;
mod document;
pub mod features;
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
