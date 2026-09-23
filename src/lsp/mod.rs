//! Purpose: Editor-independent LSP framing, sessions, documents, and server lifecycle.

mod client;
mod document;
pub(crate) mod features;
pub(crate) mod positions;
mod process;
mod rpc;
mod transport;
pub(crate) mod types;
pub(crate) mod uri;

#[cfg(test)]
mod tests;
