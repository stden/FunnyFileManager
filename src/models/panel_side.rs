//! Purpose: The identity of the active file panel.
/// Which panel is currently active: left or right
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanelSide {
    Left,
    Right,
}
