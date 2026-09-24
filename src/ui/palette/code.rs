//! Purpose: The editor's and the viewer's colours: Colorer's default scheme
//! for code, and the status bars above the editor and the viewer.

use ratatui::style::Color;

// ── Editor (Colorer's default scheme) ──────────────────────────────

/// Identifiers and plain text.
pub(in crate::ui) const CODE_NORMAL: Color = Color::Rgb(120, 200, 230);

/// Reserved words: white and bold, the way Colorer sets them apart from the
/// identifiers they surround.
pub(in crate::ui) const CODE_KEYWORD: Color = Color::Rgb(255, 255, 255);

/// String and character literals.
pub(in crate::ui) const CODE_STRING: Color = Color::Rgb(235, 185, 70);

/// Numeric literals.
pub(in crate::ui) const CODE_NUMBER: Color = Color::Rgb(120, 220, 120);

/// Comments, and the preprocessor/decorator lines that read like them.
pub(in crate::ui) const CODE_COMMENT: Color = Color::Rgb(215, 135, 65);

/// Type names and primitives, which Colorer keeps distinct from both
/// identifiers and keywords.
pub(in crate::ui) const CODE_TYPE: Color = Color::Rgb(140, 220, 200);

/// Preprocessor directives, attributes and decorators. Its own colour, not the
/// comment's: `#include` is code.
pub(in crate::ui) const CODE_DIRECTIVE: Color = Color::Rgb(150, 220, 120);

/// Operators and punctuation — white, so structure stays visible without
/// competing with the words.
pub(in crate::ui) const CODE_PUNCTUATION: Color = Color::Rgb(255, 255, 255);

/// The editor's top bar: the file name on the left, position on the right.
pub(in crate::ui) const EDITOR_BAR_BG: Color = Color::Rgb(85, 150, 216);
pub(in crate::ui) const EDITOR_BAR_FG: Color = Color::Rgb(255, 255, 255);

/// The viewer's status line: black on the same teal as the menu bar. Far
/// paints its viewer and editor status lines both this way; only the viewer
/// follows it here, because its bar is the one row of chrome the viewer has
/// and the screenshot it is drawn from shows exactly this pair.
pub(in crate::ui) const VIEWER_BAR_BG: Color = Color::Rgb(0, 128, 128);
pub(in crate::ui) const VIEWER_BAR_FG: Color = Color::Rgb(0, 0, 0);
