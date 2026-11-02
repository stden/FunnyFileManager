//! Purpose: What fills the frame behind any modal overlay: the panels, or the
//! console transcript `Ctrl+O` reveals.

/// What fills the frame behind any modal overlay.
///
/// Split out of [`Overlay`](super::Overlay) because hiding the panels is not a dialog: this
/// says *what is on screen*, while `Overlay` says *which dialog is open*. As
/// one enum the two were mutually exclusive, so the command line could not be
/// used while the console showed — and letting you keep working with the
/// output in front of you is the entire point of Far Manager's `Ctrl+O`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// The dual-panel view.
    #[default]
    Panels,
    /// `Ctrl+O`: the panels are lifted off and the console transcript shows.
    Console,
}

impl ViewMode {
    /// The other mode — what `Ctrl+O` switches to.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Panels => Self::Console,
            Self::Console => Self::Panels,
        }
    }
}
