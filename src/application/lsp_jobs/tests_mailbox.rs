//! Purpose: Preserve LSP commands under backpressure and report a lost worker.
use super::tests_session::fixture;
use super::*;
use tempfile::TempDir;

#[test]
fn full_mailbox_retries_the_command_without_losing_its_order() {
    let root = TempDir::new().expect("temp");
    let mut app = fixture(&root);
    let (manager, receiver) = crate::lsp::manager::tests_fixture::detached(1);
    let mut controller = EditorLspController {
        manager: Some(manager),
        ..Default::default()
    };
    controller.pending.extend([Command::Close, Command::Cancel]);
    controller.flush(&mut app);
    assert!(matches!(receiver.try_recv(), Ok(Command::Close)));
    assert!(matches!(controller.pending.front(), Some(Command::Cancel)));
    controller.flush(&mut app);
    assert_eq!(controller.pending.len(), 0);
    assert!(matches!(receiver.try_recv(), Ok(Command::Cancel)));
}

#[test]
fn lost_worker_fails_the_editor_and_discards_the_queued_commands() {
    let root = TempDir::new().expect("temp");
    let mut app = fixture(&root);
    let (manager, receiver) = crate::lsp::manager::tests_fixture::detached(1);
    drop(receiver);
    let mut controller = EditorLspController {
        manager: Some(manager),
        ..Default::default()
    };
    controller.pending.extend([Command::Close, Command::Cancel]);
    controller.flush(&mut app);
    assert_eq!(controller.pending.len(), 0);
    assert!(controller.manager.is_none());
    assert_eq!(
        app.get_editor().expect("editor").lsp.state,
        ServerState::Failed
    );
}
