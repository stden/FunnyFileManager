//! Purpose: Far Manager's default colour scheme, in one place.
//!
//! Every colour the interface uses is named here rather than picked at the call
//! site. Far's scheme is not decoration — it is how the panel answers "what is
//! this row" without spending a column on the answer, and that only works if
//! the same meaning always gets the same colour.
//!
//! Values are RGB rather than the sixteen ANSI names because those names are
//! whatever the user's terminal theme says they are: `Color::Blue` is navy in
//! one profile and violet in another, and a file manager that reads as Far in
//! one terminal and as something else in the next has no scheme at all.

mod aliases;
mod code;

use ratatui::style::Color;

pub(super) use aliases::*;
pub(super) use code::*;

/// The panel ground: the navy of the console palette Far was drawn for, not
/// the terminal's idea of "blue" and not a brighter one of our own.
pub(super) const BACKGROUND: Color = Color::Rgb(0, 0, 128);

/// Frames, column rules and separators.
pub(super) const FRAME: Color = Color::Rgb(85, 221, 238);

/// A directory's name — the brightest thing on the panel, because it is what
/// navigation is about.
pub(super) const DIRECTORY: Color = Color::Rgb(255, 255, 255);

/// A directory that is a Git repository with uncommitted changes.
pub(super) const DIRTY_REPO: Color = Color::Rgb(255, 190, 130);

/// An ordinary file's name.
pub(super) const FILE: Color = Color::Rgb(85, 221, 238);

/// An executable: green, the one colour Far gives a file for what it *does*
/// rather than what it is called.
pub(super) const EXECUTABLE: Color = Color::Rgb(80, 250, 123);

/// Hidden and system entries, dimmed toward the background rather than greyed:
/// grey on blue reads as damaged, this reads as recessed.
pub(super) const HIDDEN: Color = Color::Rgb(110, 142, 190);

/// Size, Date and Time. One colour for all three, so the eye reads down the
/// name column for meaning and across the data columns for facts.
pub(super) const DATA: Color = Color::Rgb(255, 255, 255);

/// A marked (selected) entry — Far's yellow, applied to the whole row.
pub(super) const MARKED: Color = Color::Rgb(255, 255, 85);

/// The cursor bar under the current entry, and the text on it.
pub(super) const CURSOR_BG: Color = Color::Rgb(85, 150, 216);
pub(super) const CURSOR_FG: Color = Color::Rgb(255, 255, 255);

/// The active panel's title, and the key bar: black on cyan.
pub(super) const HIGHLIGHT_BG: Color = Color::Rgb(85, 221, 238);
pub(super) const HIGHLIGHT_FG: Color = Color::Rgb(0, 0, 0);

// ── The command prompt ─────────────────────────────────────────────
//
// Starship's default arrangement: the parts that identify *where you are* are
// bright, and the one that says whether the last command worked is a colour
// you read without looking at.

/// The leading bullet, when the last command succeeded — and when it did not.
pub(super) const PROMPT_OK: Color = Color::Rgb(90, 160, 235);
pub(super) const PROMPT_FAILED: Color = Color::Rgb(255, 85, 85);

/// The login name and the host, with the `@` between them picked out.
///
/// Red for both: on a root shell that is the warning, and it is the same
/// colour whoever you are so the eye learns one shape.
pub(super) const PROMPT_USER: Color = Color::Rgb(224, 80, 80);
pub(super) const PROMPT_AT: Color = Color::Rgb(235, 175, 60);

/// The current directory: gold and bold, because it is what the next command
/// will act on.
pub(super) const PROMPT_PATH: Color = Color::Rgb(235, 190, 70);

/// The git branch.
pub(super) const PROMPT_BRANCH: Color = Color::Rgb(200, 140, 255);

/// `#` or `$`.
pub(super) const PROMPT_SIGIL: Color = Color::Rgb(255, 255, 255);

// ── Warning dialogs ────────────────────────────────────────────────
//
// Far gives every question that can lose data a red ground instead of the
// panel's blue. The colour is the warning: it lands before the text is read,
// which is exactly when a reflexive Enter would otherwise already be pressed.

/// The ground of a delete/discard confirmation.
pub(super) const WARNING_BG: Color = Color::Rgb(128, 0, 0);

/// Frame and text on that ground.
pub(super) const WARNING_FG: Color = Color::Rgb(255, 255, 255);

/// The shadow a dialog drops on whatever is behind it.
pub(super) const SHADOW: Color = Color::Rgb(0, 0, 0);

// ── Prompt dialogs ─────────────────────────────────────────────────
//
// Far draws a dialog that merely wants an answer on grey, not on the panel's
// blue: the panels stay visibly *behind* it, and the eye has somewhere to rest
// that is not the same colour as the file listing.

/// The ground of a prompt dialog, and the text and frame on it.
pub(super) const DIALOG_BG: Color = Color::Rgb(192, 192, 192);
pub(super) const DIALOG_FG: Color = Color::Rgb(0, 0, 0);

/// Far's help text, frame and title are black on cyan.
pub(super) const HELP_FG: Color = Color::Rgb(0, 0, 0);
pub(super) const HELP_BG: Color = Color::Rgb(0, 128, 128);

/// A control that is present but does not apply. Far dims rather than hides,
/// so the dialog keeps its shape and the user learns where the control lives.
pub(super) const DISABLED_FG: Color = Color::Rgb(128, 128, 128);

/// A text field: sunk into the dialog rather than raised out of it.
pub(super) const FIELD_BG: Color = Color::Rgb(0, 128, 128);
pub(super) const FIELD_FG: Color = Color::Rgb(0, 0, 0);
/// Text selected in a field: an offered value that the first key typed
/// replaces. White on black, the way a selected row is drawn elsewhere, so it
/// cannot be mistaken for text that typing would add to.
pub(super) const FIELD_SELECTED_BG: Color = Color::Rgb(0, 0, 0);
pub(super) const FIELD_SELECTED_FG: Color = Color::Rgb(255, 255, 255);

/// The focused form button: black on cyan; warnings use a gray button.
pub(super) const DIALOG_FOCUS_BG: Color = Color::Rgb(0, 128, 128);
pub(super) const WARNING_FOCUS_BG: Color = Color::Rgb(192, 192, 192);

/// The menu bar across the top, and the drop-downs it opens. Far draws them on
/// the same teal it uses for the key bar, so the two frames of the screen read
/// as one piece of chrome around the panels.
pub(super) const MENU_BG: Color = Color::Rgb(0, 128, 128);
pub(super) const MENU_FG: Color = Color::Rgb(0, 0, 0);
/// The heading whose drop-down is open, and the row under the cursor.
pub(super) const MENU_SELECTED_BG: Color = Color::Rgb(0, 0, 0);
pub(super) const MENU_SELECTED_FG: Color = Color::Rgb(255, 255, 255);

/// The Alt+F1 / Alt+F2 drive dialog. Far draws it on the menu's teal with a
/// white frame, rather than on the grey a form dialog gets: it is a menu that
/// happens to have columns, not a form.
pub(super) const DRIVE_FG: Color = Color::Rgb(255, 255, 255);
/// The row under the cursor, which Far inverts to the console's own black.
pub(super) const DRIVE_SELECTED_BG: Color = Color::Rgb(0, 0, 0);

/// The letter that activates a control from the keyboard — the one part of a
/// dialog that is a promise about which key to press.
pub(super) const HOTKEY: Color = Color::Rgb(255, 255, 0);
