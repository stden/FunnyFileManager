//! Purpose: Editor-independent LSP framing, sessions, documents, and server lifecycle.

use serde_json::error::Category;
use std::fmt::Display;

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

/// A `map_err` adapter for the fixed messages this module reports: it logs a
/// [`PayloadFree`] description of the original `error` as the cause of
/// `replacement`, then returns `replacement`, so the detail the message cannot
/// carry still reaches the diagnostics log.
fn logged<E: PayloadFree, R: Display>(replacement: R) -> impl FnOnce(E) -> R {
    move |error| {
        crate::diagnostics::log_error(&format!("{replacement}: {}", error.describe()));
        replacement
    }
}

/// An error [`logged`] may describe. The diagnostics log is payload-free, so
/// the description never quotes text a language server, a package manifest or
/// an install receipt supplied: that text can come from the user's documents.
trait PayloadFree: Display {
    fn describe(&self) -> String {
        self.to_string()
    }
}

// Fixed messages, at most with a byte index.
impl PayloadFree for std::str::Utf8Error {}
impl PayloadFree for std::num::ParseIntError {}
impl PayloadFree for std::path::StripPrefixError {}
impl PayloadFree for url::ParseError {}
impl PayloadFree for install::InstallError {}

/// A `serde_json` message quotes the value it rejected (`invalid type: string
/// "…"`), so only the kind of failure and, when known, its position are kept.
impl PayloadFree for serde_json::Error {
    fn describe(&self) -> String {
        let kind = match self.classify() {
            Category::Io => "I/O",
            Category::Syntax => "syntax",
            Category::Data => "data",
            Category::Eof => "end-of-input",
        };
        match self.line() {
            0 => format!("JSON {kind} error"),
            line => format!("JSON {kind} error at line {line}, column {}", self.column()),
        }
    }
}

#[cfg(test)]
mod tests;
