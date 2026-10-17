//! Purpose: Read access to the task list: its session and privilege, the
//! table and its rows, the shown view and how its last column is drawn, and
//! the refresh state.

use crate::prompt::Privilege;
use crate::scroll::ScrollLeft;

use super::{
    CommandView, Health, ProcessKey, ProcessRow, Refresh, Snapshot, Table, TaskList, TaskSession,
    ViewOptions, ViewRow,
};

impl TaskList {
    #[must_use]
    pub const fn session(&self) -> TaskSession {
        self.session
    }

    /// Whether FFM runs as root: `[` asks first unless it does.
    #[must_use]
    pub const fn privilege(&self) -> Privilege {
        self.privilege
    }

    #[must_use]
    pub const fn table(&self) -> &Table {
        &self.table
    }

    #[must_use]
    pub fn snapshot(&self) -> Option<&Snapshot> {
        match &self.table {
            Table::Ready { snapshot, .. } => Some(snapshot),
            Table::Waiting | Table::Failed(_) => None,
        }
    }

    /// How fresh the snapshot is; `None` without one.
    #[must_use]
    pub const fn health(&self) -> Option<Health> {
        match &self.table {
            Table::Ready { health, .. } => Some(*health),
            Table::Waiting | Table::Failed(_) => None,
        }
    }

    /// Every row of the snapshot, shown or not; empty without one.
    #[must_use]
    pub fn rows(&self) -> &[ProcessRow] {
        self.table.rows()
    }

    /// The row of the process `key`, if the snapshot still has it.
    #[must_use]
    pub fn row(&self, key: ProcessKey) -> Option<&ProcessRow> {
        self.rows().iter().find(|row| row.sample().key == key)
    }

    /// The shown rows, in order.
    #[must_use]
    pub fn view(&self) -> &[ViewRow] {
        &self.view
    }

    #[must_use]
    pub const fn options(&self) -> &ViewOptions {
        &self.options
    }

    #[must_use]
    pub const fn command_view(&self) -> CommandView {
        self.command_view
    }

    /// How far the last column is scrolled sideways.
    #[must_use]
    pub const fn command_scroll(&self) -> ScrollLeft {
        self.command_scroll
    }

    #[must_use]
    pub const fn refresh(&self) -> Refresh {
        self.refresh
    }
}
