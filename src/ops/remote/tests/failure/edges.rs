use super::*;

#[test]
fn a_failed_late_save_does_not_reopen_a_closed_editor() {
    let (_dir, mut app, target, source) = opened_editor();
    app.close_overlay();
    completion::apply(
        &mut app,
        save_request(target, source),
        Err(RemoteError::Conflict),
    );
    assert!(!app.overlay.is_active());
}
