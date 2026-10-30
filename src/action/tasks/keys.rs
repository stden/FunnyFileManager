//! Purpose: resolve a key to the `TaskCommand` it asks of the task list — Far's meaning first, htop's where Far has none.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{ListMotion, MaskPurpose, NiceStep, SortKey};

use super::{Pan, PanStep, TaskCommand};

/// The key as the task list reads it: a shifted character is its capital
/// with no SHIFT, however the terminal sent it — `k` with SHIFT, or `K` with
/// or without it — so `K` is never taken for `k`.
#[must_use]
pub const fn normalize(key: KeyEvent) -> KeyEvent {
    match key.code {
        KeyCode::Char(ch) if key.modifiers.contains(KeyModifiers::SHIFT) => KeyEvent {
            code: KeyCode::Char(ch.to_ascii_uppercase()),
            modifiers: key.modifiers.difference(KeyModifiers::SHIFT),
            ..key
        },
        _ => key,
    }
}

/// What `key` asks of the task list in Browse mode.
#[must_use]
pub const fn resolve(key: KeyEvent) -> TaskCommand {
    let key = normalize(key);
    keymap! { key;
        KeyCode::Char(ch) if has(CONTROL) => control(ch),
        KeyCode::Char(ch) if has(ALT) => TaskCommand::Search(Some(ch)),
        KeyCode::Char(ch) => character(ch),
        KeyCode::F(number) if exact(CONTROL) => control_function_key(number),
        KeyCode::F(number) if without(CONTROL + ALT + SHIFT) => function_key(number),
        _ => named(key),
    }
}

/// Ctrl+letter. Ctrl+C closes the list rather than quitting ffm.
const fn control(ch: char) -> TaskCommand {
    match ch {
        'c' | 'C' | 'w' | 'W' => TaskCommand::Close,
        't' | 'T' => TaskCommand::ToggleTree,
        'r' | 'R' | 'l' | 'L' => TaskCommand::RefreshNow,
        _ => TaskCommand::Ignore,
    }
}

/// A character without Control or Alt: marking, finding and closing.
const fn character(ch: char) -> TaskCommand {
    match ch {
        ' ' => TaskCommand::ToggleMark,
        'c' => TaskCommand::MarkFamily,
        'U' => TaskCommand::UnmarkAll,
        '*' => TaskCommand::InvertMarks,
        '+' => TaskCommand::MarkByMask(MaskPurpose::Select),
        '-' => TaskCommand::MarkByMask(MaskPurpose::Unselect),
        '\\' => TaskCommand::Filter,
        '/' => TaskCommand::Search(None),
        '0'..='9' => TaskCommand::PidJump(ch),
        'h' | '?' => TaskCommand::Help,
        'q' => TaskCommand::Close,
        _ => htop_letter(ch),
    }
}

/// htop's letters for what is shown, its order, the refresh, and the
/// processes themselves.
const fn htop_letter(ch: char) -> TaskCommand {
    match ch {
        't' => TaskCommand::ToggleTree,
        'u' => TaskCommand::UserMenu,
        'K' => TaskCommand::ToggleKernelThreads,
        '<' | '>' => TaskCommand::SortMenu,
        'P' => TaskCommand::SortBy(SortKey::Cpu),
        'M' => TaskCommand::SortBy(SortKey::Resident),
        'T' => TaskCommand::SortBy(SortKey::CpuTime),
        'N' => TaskCommand::SortBy(SortKey::Pid),
        'I' => TaskCommand::InvertOrder,
        'Z' => TaskCommand::ToggleHold,
        ']' => TaskCommand::Nice(NiceStep::Raise),
        '[' => TaskCommand::Nice(NiceStep::Lower),
        'k' => TaskCommand::SignalMenu,
        _ => TaskCommand::Ignore,
    }
}

/// A bare F-key: the key bar's labels.
const fn function_key(number: u8) -> TaskCommand {
    match number {
        1 => TaskCommand::Help,
        2 => TaskCommand::ToggleCommandView,
        3 => TaskCommand::Info,
        4 => TaskCommand::Filter,
        5 => TaskCommand::ToggleTree,
        6 => TaskCommand::SortMenu,
        7 => TaskCommand::Priority,
        8 => TaskCommand::Terminate,
        9 => TaskCommand::SignalMenu,
        10 => TaskCommand::Close,
        11 => TaskCommand::ToggleHold,
        12 => TaskCommand::ToggleTab,
        _ => TaskCommand::Ignore,
    }
}

/// Ctrl+F-key: the panel's direct sort keys, each on the process column
/// that answers the same question.
const fn control_function_key(number: u8) -> TaskCommand {
    match number {
        3 => TaskCommand::SortBy(SortKey::Name),
        6 => TaskCommand::SortBy(SortKey::Resident),
        7 => TaskCommand::SortBy(SortKey::Pid),
        8 => TaskCommand::SortBy(SortKey::Started),
        10 => TaskCommand::SortBy(SortKey::Command),
        11 => TaskCommand::SortBy(SortKey::User),
        12 => TaskCommand::SortMenu,
        _ => TaskCommand::Ignore,
    }
}

/// The keys with a name rather than a character.
const fn named(key: KeyEvent) -> TaskCommand {
    keymap! { key;
        KeyCode::Up => TaskCommand::Move(ListMotion::Up),
        KeyCode::Down => TaskCommand::Move(ListMotion::Down),
        KeyCode::PageUp => TaskCommand::Move(ListMotion::PageUp),
        KeyCode::PageDown => TaskCommand::Move(ListMotion::PageDown),
        KeyCode::Home => TaskCommand::Move(ListMotion::Home),
        KeyCode::End => TaskCommand::Move(ListMotion::End),
        KeyCode::Left if has(CONTROL) => TaskCommand::Pan(Pan::Left(PanStep::Wide)),
        KeyCode::Right if has(CONTROL) => TaskCommand::Pan(Pan::Right(PanStep::Wide)),
        KeyCode::Left => TaskCommand::Pan(Pan::Left(PanStep::Cell)),
        KeyCode::Right => TaskCommand::Pan(Pan::Right(PanStep::Cell)),
        KeyCode::Insert => TaskCommand::ToggleMark,
        KeyCode::Enter => TaskCommand::Info,
        KeyCode::Tab if without(CONTROL + ALT + SHIFT) => TaskCommand::ToggleTab,
        KeyCode::Delete if without(CONTROL + ALT + SHIFT) => TaskCommand::Terminate,
        KeyCode::Esc => TaskCommand::Close,
        _ => TaskCommand::Ignore,
    }
}
