//! Purpose: `PathSyntax` — how the host spells a path separator and a shell word, passed to the pure completion helpers so both spellings are testable on every OS.

/// How the host spells a path and quotes a shell word.
///
/// Passed explicitly rather than read from `cfg!` inside each helper: a Linux
/// test can then exercise the Windows rules, and a Windows test the Posix
/// ones. Production always passes [`PathSyntax::HOST`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathSyntax {
    /// `/` separates; a backslash is an ordinary filename character to the
    /// filesystem and an escape to `sh`.
    Posix,
    /// `\` and `/` both separate, `C:` names a drive, and `cmd` has no
    /// backslash escape — a word is protected by double quotes instead.
    Windows,
}

impl PathSyntax {
    /// The syntax of the host this binary runs on.
    pub const HOST: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Posix
    };

    /// The separator the syntax writes by default: `std::path::MAIN_SEPARATOR`
    /// of the host it describes.
    #[must_use]
    pub const fn main_separator(self) -> char {
        match self {
            Self::Posix => '/',
            Self::Windows => '\\',
        }
    }

    /// Whether `ch` separates path components.
    #[must_use]
    pub const fn is_separator(self, ch: char) -> bool {
        match self {
            Self::Posix => ch == '/',
            Self::Windows => matches!(ch, '/' | '\\'),
        }
    }

    /// The separator to append to a directory candidate: the one the user
    /// typed last in `token`, so `C:/work/` stays forward-slashed on Windows,
    /// else the syntax's [`main_separator`](Self::main_separator).
    #[must_use]
    pub fn dir_suffix(self, token: &str) -> char {
        token
            .chars()
            .rev()
            .find(|&ch| self.is_separator(ch))
            .unwrap_or_else(|| self.main_separator())
    }

    /// Whether `text` ends in a separator — a directory the user will descend
    /// into rather than a finished argument.
    #[must_use]
    pub fn ends_with_separator(self, text: &str) -> bool {
        text.chars()
            .next_back()
            .is_some_and(|ch| self.is_separator(ch))
    }

    /// The root a bare drive letter names — `C:\` for `C:` — so `cd C:` has
    /// something to complete to: `std::path` calls `C:` relative and would
    /// join it onto the panel directory. Posix has no drives.
    #[must_use]
    pub fn drive_root(self, token: &str) -> Option<String> {
        if self != Self::Windows {
            return None;
        }
        let mut chars = token.chars();
        let is_drive =
            chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) && chars.as_str() == ":";
        is_drive.then(|| format!("{token}\\"))
    }

    /// Split a token being typed into its already-typed directory part (ending
    /// in a separator, or empty) and the partial leaf name being completed.
    #[must_use]
    pub fn split_token(self, token: &str) -> (&str, &str) {
        match token.rfind(|ch| self.is_separator(ch)) {
            // Every separator is one ASCII byte, so the split lands on a char
            // boundary.
            Some(i) => token.split_at(i + 1),
            None => ("", token),
        }
    }

    /// Split a path completion *candidate* (not a token being typed) into its
    /// directory part and leaf, ignoring the candidate's own trailing
    /// separator when finding the split point — so `"sub/handlers/"` splits as
    /// `("sub/", "handlers/")`, not `("sub/handlers/", "")`.
    #[must_use]
    pub fn split_leaf(self, candidate: &str) -> (&str, &str) {
        let searched = candidate
            .strip_suffix(|ch| self.is_separator(ch))
            .unwrap_or(candidate);
        match searched.rfind(|ch| self.is_separator(ch)) {
            Some(i) => candidate.split_at(i + 1),
            None => ("", candidate),
        }
    }
}

#[cfg(test)]
mod tests;
