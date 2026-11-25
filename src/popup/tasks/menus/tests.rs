use super::choose_with;
use crate::models::{MenuCursor, Modal, Operation, TaskMenu};
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};

#[test]
fn choose_with_empty_offered_signals_leaves_browse_modal() {
    let mut tasks = TaskApp::open(vec![named(10, "vim")], Privilege::User);
    let targets = tasks.list().freeze_targets(Operation::Signal);
    choose_with(
        tasks.list(),
        TaskMenu::Signal {
            targets,
            cursor: MenuCursor::at(0, 1).unwrap(),
        },
        &[],
    );
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}
