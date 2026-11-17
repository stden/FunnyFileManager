//! Purpose: A name the database reports or a config file gives: a connection,
//! a schema, a table or a column. It is never empty.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Name(String);

impl Name {
    /// The name, or `None` when `text` is empty.
    pub(crate) fn new(text: impl Into<String>) -> Option<Self> {
        let text = text.into();
        (!text.is_empty()).then_some(Self(text))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}
