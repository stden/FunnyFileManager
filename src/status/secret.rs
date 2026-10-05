//! Purpose: `Secret` — which vault field a clipboard action handled, named by its label and never its value.

/// Which secret a clipboard action handled, named by its field label.
///
/// A discriminant instead of the copied text: a `Status` renders straight
/// into the status bar, so a variant that carried the value would put the
/// secret on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    Password,
    Username,
}

impl Secret {
    /// The on-screen field label — the only rendering a secret gets.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::Username => "User name",
        }
    }
}
