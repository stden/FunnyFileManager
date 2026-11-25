use super::{find, jump};
use crate::models::{Modal, SearchFrom};
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};

#[test]
fn find_and_jump_ignore_non_matching_modal_or_empty_pid() {
    let mut tasks = TaskApp::open(vec![named(10, "vim"), named(20, "sh")], Privilege::User);
    find(tasks.list(), "sh", SearchFrom::Cursor, 10);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    jump(tasks.list(), None, 10);
    assert_eq!(tasks.list().cursor().map(|c| c.key().pid().get()), Some(20));
}
