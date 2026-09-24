//! Purpose: The task list's keys while nothing is open on it: every
//! [`TaskCommand`] the keymap resolves, applied to the list.
//!
//! The match is exhaustive, so a command added to the keymap cannot be
//! forgotten here. Nothing in it sends a signal: F8, Del, F9 and `k` only
//! open a dialog, and `]` and `[` renice through the same checks as F7.

use crossterm::event::KeyEvent;

use crate::action::tasks::{resolve, Pan, TaskCommand};
use crate::models::{App, ControlRefusal, MaskPrompt, MaskPurpose, Modal, Notice, TaskList};
use crate::ops::tasks::control;
use crate::scroll::ScrollTop;

use super::{bars, menus};

pub(super) fn process(app: &mut App, key: KeyEvent) {
    match resolve(key) {
        TaskCommand::Close => app.close_overlay(),
        TaskCommand::Nice(step) => control::step_nice(app, step),
        command => {
            let viewport = app.viewport;
            if let Some(list) = app.task_list_mut() {
                let page = list.page_for(viewport);
                apply(list, command, page);
            }
        }
    }
}

/// `command` on a list whose table shows `page` rows.
fn apply(list: &mut TaskList, command: TaskCommand, page: usize) {
    match command {
        TaskCommand::Move(motion) => list.move_cursor(motion, page),
        TaskCommand::Pan(Pan::Left(step)) => list.pan_left(step.cells()),
        TaskCommand::Pan(Pan::Right(step)) => list.pan_right(step.cells()),
        TaskCommand::ToggleMark => list.toggle_mark(page),
        TaskCommand::MarkFamily => list.mark_family(),
        TaskCommand::UnmarkAll => list.unmark_all(),
        TaskCommand::InvertMarks => list.invert_marks(),
        TaskCommand::MarkByMask(purpose) => open_mask(list, purpose),
        TaskCommand::Info => open_info(list),
        TaskCommand::Help => list.set_modal(Modal::Help(ScrollTop::TOP)),
        TaskCommand::ToggleCommandView => list.toggle_command_view(),
        TaskCommand::Filter => bars::open_filter(list),
        TaskCommand::Search(typed) => bars::open_search(list, typed, page),
        TaskCommand::PidJump(digit) => bars::open_pid_jump(list, digit, page),
        TaskCommand::ToggleTree => list.toggle_tree(),
        TaskCommand::SortMenu => menus::open_sort(list),
        TaskCommand::SortBy(key) => list.choose_sort(key),
        TaskCommand::InvertOrder => list.invert_order(),
        TaskCommand::UserMenu => menus::open_user(list),
        TaskCommand::ToggleKernelThreads => list.toggle_kernel_threads(),
        TaskCommand::Priority => control::open_priority(list),
        TaskCommand::Terminate => control::confirm_terminate(list),
        TaskCommand::SignalMenu => control::open_signal_menu(list),
        TaskCommand::ToggleHold => list.toggle_hold(),
        TaskCommand::RefreshNow => list.refresh_now(),
        TaskCommand::Nice(_) | TaskCommand::Close | TaskCommand::Ignore => {}
    }
}

/// Gray `+` and `-`: the mask dialog, offering the cursor process's short
/// name, as Far offers the file under the cursor; `*` on an empty list.
fn open_mask(list: &mut TaskList, purpose: MaskPurpose) {
    let offered = cursor_name(list).unwrap_or_else(|| "*".to_owned());
    list.set_modal(Modal::Mask(MaskPrompt::new(purpose, &offered)));
}

/// The short name of the process under the cursor.
fn cursor_name(list: &TaskList) -> Option<String> {
    let key = list.cursor()?.key();
    list.row(key).map(|row| row.sample().name.clone())
}

/// Enter and F3: the cursor process's information, or why there is none.
fn open_info(list: &mut TaskList) {
    match list.cursor() {
        Some(cursor) => list.set_modal(Modal::Info(cursor.key())),
        None => list.set_notice(Notice::Refused(ControlRefusal::NoTarget)),
    }
}
