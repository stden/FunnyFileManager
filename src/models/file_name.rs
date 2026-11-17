//! Purpose: `FileName`, a bare directory-entry name that is never a path.

use std::path::Path;

/// A bare directory-entry name — never a path.
///
/// Marks are name-keyed, and a mark only means anything relative to the
/// directory that was listed when it was made; that is why `navigate_to` drops
/// them. Storing them as plain `String`s left that distinction to a comment,
/// and a comment cannot stop a `PathBuf::display().to_string()` being inserted
/// into the set. `Borrow<str>` keeps lookups by `&str` allocation-free, and the
/// derived `Ord` matches `str`'s because the type is one `String`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileName(String);

impl FileName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for FileName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FileName {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for FileName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl AsRef<Path> for FileName {
    /// A bare name is a valid single-component path, which is what makes
    /// `panel.path.join(name)` the one correct way to turn a mark into a path.
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl std::fmt::Display for FileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
