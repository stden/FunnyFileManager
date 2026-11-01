//! Purpose: Names for what a colour means in one dialog — the Language
//! servers dialog's statuses and the task list's htop meters — each an alias
//! of a scheme colour that already means the same, so neither adds a colour.

use ratatui::style::Color;

use super::{EXECUTABLE, FRAME, MARKED, PROMPT_BRANCH, PROMPT_FAILED, PROMPT_OK};

// ── Language servers dialog ────────────────────────────────────────
//
// Names for what a colour means there, each an existing colour, so the dialog
// adds no new ones.

/// A server that will start: the green an executable file has.
pub(in crate::ui) const STATUS_OK: Color = EXECUTABLE;
/// A server waiting for an install, or running the recipe's program instead:
/// the yellow of a mark.
pub(in crate::ui) const STATUS_PENDING: Color = MARKED;
/// A server that cannot start, and a problem that blocks a save: the red of a
/// failed command.
pub(in crate::ui) const STATUS_FAILED: Color = PROMPT_FAILED;
/// The completion dropdown's frame and highlight, shared by the command line
/// and the dialog's form.
pub(in crate::ui) const COMPLETION_BG: Color = FRAME;

// ── The task list's meters ─────────────────────────────────────────
//
// htop's meter colours, each an alias of the scheme colour that already
// means the same hue, so the meters never add a colour of their own.

/// CPU time in user code, and memory in use: green.
pub(in crate::ui) const METER_USER: Color = EXECUTABLE;
/// CPU time at a changed nice value, and buffer memory: blue.
pub(in crate::ui) const METER_NICE: Color = PROMPT_OK;
/// CPU time in the kernel, and swap in use: red.
pub(in crate::ui) const METER_KERNEL: Color = PROMPT_FAILED;
/// CPU time stolen by or given to virtual machines: cyan.
pub(in crate::ui) const METER_VIRT: Color = FRAME;
/// Shared memory: magenta.
pub(in crate::ui) const METER_SHARED: Color = PROMPT_BRANCH;
/// The page cache and the swap cache: yellow.
pub(in crate::ui) const METER_CACHE: Color = MARKED;
/// The `D` and `Z` state letters: a process stuck in the kernel, or dead
/// and not yet reaped.
pub(in crate::ui) const STATE_ALERT: Color = PROMPT_FAILED;
