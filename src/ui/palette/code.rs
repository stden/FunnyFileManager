//! Purpose: The editor's and the viewer's colours: Colorer's console scheme
//! for code, and the status bar above the editor and the viewer.
//!
//! The values are the console's sixteen colours, not blends of our own: on
//! Far's navy ground they are what Colorer draws and what a Far user reads by.

use ratatui::style::Color;

// ── Code (Colorer's console scheme) ─────────────────────────────────

/// Identifiers and plain text: light cyan.
pub(in crate::ui) const CODE_NORMAL: Color = Color::Rgb(0, 255, 255);

/// Reserved words: white and bold, the way Colorer sets them apart from the
/// identifiers they surround.
pub(in crate::ui) const CODE_KEYWORD: Color = Color::Rgb(255, 255, 255);

/// String and character literals: yellow.
pub(in crate::ui) const CODE_STRING: Color = Color::Rgb(255, 255, 0);

/// Numeric literals and command-line switches: light magenta.
pub(in crate::ui) const CODE_NUMBER: Color = Color::Rgb(255, 0, 255);

/// Comments: dark cyan, quieter than the code around them.
pub(in crate::ui) const CODE_COMMENT: Color = Color::Rgb(0, 128, 128);

/// Type names and primitives, kept distinct from the identifiers they would
/// otherwise blend into: light green.
pub(in crate::ui) const CODE_TYPE: Color = Color::Rgb(0, 255, 0);

/// Preprocessor directives, attributes and decorators: light green, Colorer's
/// directive colour. Not the comment's: `#include` is code.
pub(in crate::ui) const CODE_DIRECTIVE: Color = Color::Rgb(0, 255, 0);

/// Operators and punctuation — white, so structure stays visible without
/// competing with the words.
pub(in crate::ui) const CODE_PUNCTUATION: Color = Color::Rgb(255, 255, 255);

/// Markup element names and `=` delimiters: light grey (`#C0C0C0`).
pub(in crate::ui) const CODE_TAG: Color = Color::Rgb(192, 192, 192);

/// Hyperlinks inside markup attributes/comments and unclosed tag openings:
/// bright red (`#FF0000`).
pub(in crate::ui) const CODE_URL: Color = Color::Rgb(255, 0, 0);

/// Bare URI in markup element text (`<url>http://...</url>`): black on dark red.
pub(in crate::ui) const CODE_URI_FG: Color = Color::Rgb(0, 0, 0);
pub(in crate::ui) const CODE_URI_BG: Color = Color::Rgb(128, 0, 0);

/// The status bar above the editor and the viewer: the file on the left, the
/// position on the right, black on the teal of the menu bar, as Far paints
/// both.
pub(in crate::ui) const STATUS_BAR_BG: Color = Color::Rgb(0, 128, 128);
pub(in crate::ui) const STATUS_BAR_FG: Color = Color::Rgb(0, 0, 0);
