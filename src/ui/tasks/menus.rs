//! Purpose: The task list's three menus — Sort by, Send signal and Show
//! processes of — as a title and rows for the shared choices menu, built
//! once for the drawing and the clicks, so a click lands on the row drawn.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::models::{Owner, Signal, SortKey, Target, TaskList, TaskMenu, UserScope};

use super::super::menu_dialog::choices;
use super::super::panel::pad_cells;
use super::super::{printable_name, Cells};
use super::format;

/// The tick on the choice in force, and the blank the others have.
const TICK: &str = "√";
const UNTICKED: &str = " ";
/// Blanks between a row's label and its shortcut or purpose.
const COLUMN_GAP: &str = "   ";
/// The Sort by menu's last row.
const REVERSE: (&str, &str) = ("Reverse order", "I");

/// A menu as drawn: its title, its rows and the row under the cursor.
pub(super) struct MenuView {
    pub(super) title: String,
    pub(super) rows: Vec<String>,
    pub(super) selected: usize,
}

impl MenuView {
    /// `menu`, open on `list`.
    pub(super) fn of(list: &TaskList, menu: &TaskMenu) -> Self {
        let (title, rows) = match menu {
            TaskMenu::Sort(_) => ("Sort by".to_owned(), sort_rows(list)),
            TaskMenu::Signal { targets, .. } => (signal_title(targets.targets()), signal_rows()),
            TaskMenu::User { owners, .. } => {
                ("Show processes of".to_owned(), user_rows(list, owners))
            }
        };
        Self {
            title,
            rows,
            selected: menu.cursor().index(),
        }
    }

    pub(super) fn render(&self, f: &mut Frame<'_>) {
        choices::render(f, &self.title, &self.rows, self.selected);
    }

    /// The row under (`column`, `row`) of a `screen`, if a row is there.
    pub(super) fn row_at(&self, screen: Rect, column: u16, row: u16) -> Option<usize> {
        choices::row_at(screen, &self.title, &self.rows, self.selected, column, row)
    }
}

/// The name each sort key has in the menu, and the key that sorts by it
/// directly, where one does.
const fn sort_entry(key: SortKey) -> (&'static str, &'static str) {
    match key {
        SortKey::Pid => ("PID", "N"),
        SortKey::User => ("User", "Ctrl+F11"),
        SortKey::Priority => ("Priority", ""),
        SortKey::Virtual => ("Virtual memory", ""),
        SortKey::Resident => ("Resident memory", "M"),
        SortKey::Shared => ("Shared memory", ""),
        SortKey::State => ("State", ""),
        SortKey::Cpu => ("CPU%", "P"),
        SortKey::CpuTime => ("CPU time", "T"),
        SortKey::Started => ("Start time", "Ctrl+F8"),
        SortKey::Name => ("Name", "Ctrl+F3"),
        SortKey::Command => ("Command", "Ctrl+F10"),
    }
}

/// One row per sort key, the key in force ticked, then Reverse order.
fn sort_rows(list: &TaskList) -> Vec<String> {
    let current = list.options().sort.key();
    let keys = SortKey::ALL
        .iter()
        .map(|key| (tick(key, &current), sort_entry(*key)));
    let entries: Vec<_> = keys.chain([(UNTICKED, REVERSE)]).collect();
    let width = widest(entries.iter().map(|(_, (label, _))| *label));
    entries
        .into_iter()
        .map(|(tick, (label, shortcut))| {
            let row = format!("{tick} {}{COLUMN_GAP}{shortcut}", pad_cells(label, width));
            row.trim_end().to_owned()
        })
        .collect()
}

/// `Send signal to 4242 firefox`, or `… to 3 processes`.
fn signal_title(targets: &[Target]) -> String {
    match targets {
        [only] => format!(
            "Send signal to {} {}",
            only.key().pid(),
            printable_name(only.name())
        ),
        many => format!("Send signal to {} processes", many.len()),
    }
}

/// One row per offered signal: its name, then what it asks.
fn signal_rows() -> Vec<String> {
    let width = widest(Signal::OFFERED.iter().map(|signal| signal.name()));
    Signal::OFFERED
        .iter()
        .map(|signal| {
            format!(
                "{}{COLUMN_GAP}{}",
                pad_cells(signal.name(), width),
                signal.purpose()
            )
        })
        .collect()
}

/// All users, then each owner with how many processes they have now; the
/// scope in force ticked.
fn user_rows(list: &TaskList, owners: &[Owner]) -> Vec<String> {
    let scope = &list.options().scope;
    let everyone = format!("{} All users", tick(&UserScope::All, scope));
    let each = owners.iter().map(|owner| {
        let count = list
            .rows()
            .iter()
            .filter(|row| row.sample().owner == *owner)
            .count();
        let tick = tick(&UserScope::Only(owner.clone()), scope);
        format!("{tick} {} ({count})", format::owner(owner))
    });
    std::iter::once(everyone).chain(each).collect()
}

/// The tick column: `√` on `choice` when it is the one `in_force`, blank
/// elsewhere.
fn tick<T: PartialEq>(choice: &T, in_force: &T) -> &'static str {
    if choice == in_force {
        TICK
    } else {
        UNTICKED
    }
}

/// The widest of `labels`.
fn widest<'a>(labels: impl Iterator<Item = &'a str>) -> Cells {
    labels.map(Cells::of).max().unwrap_or(Cells::ZERO)
}
