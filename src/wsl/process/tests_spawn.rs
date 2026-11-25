//! Purpose: Verify pipe thread spawn failure formatting.

use super::finish_pipe_spawn;

#[test]
fn finish_pipe_spawn_formats_error_reason() {
    let (_sender, receiver) = std::sync::mpsc::sync_channel(1);
    let err =
        finish_pipe_spawn::<()>(Err(std::io::Error::other("thread limit")), receiver).unwrap_err();
    assert!(err.contains("Cannot capture WSL output: thread limit"));
}
