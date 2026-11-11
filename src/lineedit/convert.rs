//! Purpose: `LineEditor` as plain text for serde and `From`, leaving the caret,
//! the selection and the kill slot out of what is saved.

use super::LineEditor;

/// Saved and sent as its text alone: the caret, the selection and the kill
/// slot belong to the editing, not to what was typed.
impl serde::Serialize for LineEditor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

impl<'de> serde::Deserialize<'de> for LineEditor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl From<&str> for LineEditor {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for LineEditor {
    fn from(text: String) -> Self {
        Self {
            cursor: text.len(),
            text,
            anchor: None,
            kill: String::new(),
        }
    }
}
