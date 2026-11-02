//! Purpose: the key tables — every action of every context of `ffm.yml`.
//!
//! A row names an action, its command, its default keys, where the key
//! comes from and what the action does. The defaults are Far Manager's
//! wherever Far has the function.

/// The six rows every list shares, with the commands of `$command`: the arrows,
/// the paging keys, Home and End. Home and End name the list's first and last
/// rows, so their notes come in as arguments.
/// The six rows every list shares, with the commands of `$command`: the arrows,
/// the paging keys, Home and End. Home and End name the list's first and last
/// rows, so their notes come in as arguments; a list's own rows follow them.
macro_rules! list_moves {
    ($command:ident, $first:literal, $last:literal) => {
        list_moves!($command, $first, $last, [])
    };
    ($command:ident, $first:literal, $last:literal, [$($extra:expr),* $(,)?] $(,)?) => {
        [
            crate::keymap::row(
                "cursor_up",
                $command::Move(crate::models::ListMotion::Up),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::Up,
                )],
            )
            .note(crate::keymap::Origin::FarSame, "Move the cursor up"),
            crate::keymap::row(
                "cursor_down",
                $command::Move(crate::models::ListMotion::Down),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::Down,
                )],
            )
            .note(crate::keymap::Origin::FarSame, "Move the cursor down"),
            crate::keymap::row(
                "page_up",
                $command::Move(crate::models::ListMotion::PageUp),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::PgUp,
                )],
            )
            .note(
                crate::keymap::Origin::FarSame,
                "Move the cursor one screen up",
            ),
            crate::keymap::row(
                "page_down",
                $command::Move(crate::models::ListMotion::PageDown),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::PgDn,
                )],
            )
            .note(
                crate::keymap::Origin::FarSame,
                "Move the cursor one screen down",
            ),
            crate::keymap::row(
                "cursor_home",
                $command::Move(crate::models::ListMotion::Home),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::Home,
                )],
            )
            .note(crate::keymap::Origin::FarSame, $first),
            crate::keymap::row(
                "cursor_end",
                $command::Move(crate::models::ListMotion::End),
                &[crate::keymap::chords::named(
                    crate::action::chord::NamedKey::End,
                )],
            )
            .note(crate::keymap::Origin::FarSame, $last),
            $($extra),*
        ]
    };
}

mod command_line;
mod editor;
mod language_servers;
mod netbox_panel;
mod panels;
mod path_editor;
mod task_list;
mod tree_panel;
mod vault;
mod viewer;
mod wsl_manager;

pub use command_line::{COMMAND_LINE, COMMAND_LINE_COMPLETION, COMMAND_LINE_SEARCH};
pub use editor::EDITOR;
pub use language_servers::LANGUAGE_SERVERS;
pub use netbox_panel::NETBOX_PANEL;
pub use panels::PANELS;
pub use path_editor::PATH_EDITOR;
pub use task_list::{TASK_LIST, TASK_LIST_SEARCH};
pub use tree_panel::TREE_PANEL;
pub use vault::VAULT;
pub use viewer::VIEWER;
pub use wsl_manager::WSL_MANAGER;
