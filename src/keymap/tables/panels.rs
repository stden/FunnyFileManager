//! Purpose: the `panels` table — the file panels and the Ctrl+O console:
//! the cursor, marking, every function key with and without a modifier, the
//! view and sort modes, and every F9 row, keyless ones included.

use crate::action::Action;
use crate::keymap::{Context, Table};

mod control;
mod function_keys;
mod keys;
mod menu_rows;

pub const PANELS: Table<Action> = Table {
    context: Context::Panels,
    groups: &[
        keys::MOVING,
        keys::MARKING,
        function_keys::BARE,
        function_keys::SHIFT,
        function_keys::ALT,
        control::PANELS_AND_SORTING,
        control::VIEW_MODES,
        control::COMMANDS,
        keys::COMMAND_LINE,
        menu_rows::KEYLESS,
        menu_rows::OPTIONS,
    ],
};
