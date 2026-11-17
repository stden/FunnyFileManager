use super::{mutate, RemoteTask};
use crate::remote::RemoteError;

#[test]
fn a_refresh_cannot_be_executed_as_a_filesystem_mutation() {
    assert_eq!(
        mutate(&RemoteTask::Refresh(Vec::new())),
        Err(RemoteError::Failed("not a filesystem mutation"))
    );
}
