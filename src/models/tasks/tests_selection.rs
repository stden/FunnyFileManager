//! Purpose: Tests for the selection: moves and the scroll offset, a cursor
//! that follows its process or keeps its row, marks by key, family, mask and
//! inversion, the wrapping search and the PID jump.

use super::tests_support::{child_of, key, listing, pid, process, resample, shown};
use super::{CommandView, Cursor, Found, ListMotion, Notice, ProcessSample, SearchFrom, TaskList};
use crate::mask::Mask;
use crate::models::MaskPurpose;
use crate::scroll::visible_window;

mod tests_cursor;
mod tests_marks;
mod tests_search;

const PAGE: usize = 5;

/// Twenty processes, PIDs 1 to 20, named `p1` to `p20`.
fn twenty() -> TaskList {
    listing(
        (1..=20)
            .map(|raw| process(raw, &format!("p{raw}")))
            .collect(),
    )
}

fn cursor_pid(list: &TaskList) -> Option<u32> {
    list.cursor().map(|cursor| cursor.key().pid().get())
}
