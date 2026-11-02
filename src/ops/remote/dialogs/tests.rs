use super::{task, RemoteError, RemoteInput};

#[test]
fn selection_dialog_cannot_create_a_remote_worker_job() {
    assert!(matches!(
        task(RemoteInput::Selection { select: true }, "*"),
        Err(RemoteError::InvalidInput("selection is not a job"))
    ));
}
