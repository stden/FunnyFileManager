//! Purpose: Asking the table for a column past its last, or for a row the list
//! does not hold, draws nothing there.

use super::*;
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{pid, TaskApp};

#[test]
fn a_column_past_the_last_one_is_not_there() {
    let columns = Columns::for_list(&TaskList::default(), Cells::new(40));
    assert_eq!(columns.at(1_000), None);
}

#[test]
fn a_row_the_list_does_not_hold_draws_as_a_blank_line() {
    let mut app = TaskApp::open(vec![process(pid(10), "sh")], Privilege::User);
    let shown = app.list().view()[0];
    let columns = Columns::for_list(app.list(), Cells::new(40));
    let empty = TaskList::default();
    assert_eq!(columns.row(&empty, shown, RowBar::Plain), columns.empty());
}
