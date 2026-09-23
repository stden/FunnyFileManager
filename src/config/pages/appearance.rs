//! Purpose: The pages that decide how the screen looks: interface, colours,
//! and files highlighting.

use serde::{Deserialize, Serialize};

/// Interface settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent Far interface toggles, each saved as its own true/false"
)]
pub struct InterfaceSettings {
    pub clock: bool,
    pub key_bar: bool,
    /// The menu bar always on screen, not only while `F9` holds it open —
    /// which is how the bar is drawn today, so the default says so.
    pub menu_bar: bool,
    pub mouse: bool,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            clock: true,
            key_bar: true,
            menu_bar: true,
            mouse: true,
        }
    }
}

/// The colour schemes `ui/palette.rs` can draw with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    FarBlue,
    Black,
    Grey,
}

/// Colors.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorSettings {
    pub theme: Theme,
}

/// Files highlighting: mask → colour name, consumed by the panel's row colour.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightingSettings {
    pub rules: Vec<(String, String)>,
}
