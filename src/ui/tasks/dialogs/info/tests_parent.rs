//! Purpose: The info row names the parent of a process, by its PID.

use super::*;
use crate::models::ProcessSample;
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{pid, TaskApp};

#[test]
fn the_info_lines_name_the_parent_process() {
    let child = ProcessSample {
        parent: Some(pid(1)),
        ..process(pid(10), "sh")
    };
    let mut app = TaskApp::open(vec![child], Privilege::User);
    let list = app.list();
    let row = &list.rows()[0];
    assert!(
        info_lines(row)
            .iter()
            .any(|line| line.contains("Parent") && line.contains('1')),
        "{:?}",
        info_lines(row)
    );
}
