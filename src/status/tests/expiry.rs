//! Purpose: how long a status message stays on the info row as the cursor moves.

use super::*;

/// Far's info row carries a message only until the user moves on. Before this,
/// an error sat in place of the selected entry's name for every later frame,
/// until some other operation happened to replace it.
#[test]
fn a_message_survives_its_own_frame_and_goes_when_the_cursor_moves() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::Error("Permission denied".to_owned()));
    assert!(
        app.status_news().is_some(),
        "the message must exist to start"
    );

    // The frame that shows it. A message set since the last frame is tied to
    // the cursor this one draws, so it cannot be cleared before it is seen.
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "the message was cleared before it reached the screen"
    );

    // A second frame with the cursor unmoved still shows it: only moving away
    // ends it, not the mere passage of frames.
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "an idle frame ended the message"
    );

    app.get_active_panel().next();
    app.expire_status_on_move();
    assert!(
        app.status_news().is_none(),
        "the entry name must come back once the cursor moves"
    );
}

/// Switching panels is moving on too: the other panel's row is a different
/// place to be looking, so the message belongs to neither any more.
#[test]
fn a_message_does_not_follow_the_cursor_to_the_other_panel() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::Error("Permission denied".to_owned()));
    app.expire_status_on_move();
    app.switch_panel();
    app.expire_status_on_move();

    assert!(app.status_news().is_none());
}

/// The info row and the command prompt read the same field. Expiring a message
/// is a presentation decision for the row only: a command that failed must not
/// start reporting success because the cursor moved.
#[test]
fn expiry_does_not_rewrite_the_prompts_verdict() {
    use crate::prompt::LastCommand;

    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.set_status(Status::SaveFailed {
        error: "permission denied".to_owned(),
    });
    app.expire_status_on_move();
    app.get_active_panel().next();
    app.expire_status_on_move();

    assert!(app.status_news().is_none(), "the row gave the name back");
    assert_eq!(
        LastCommand::from_status(&app.status),
        LastCommand::Failed,
        "the prompt forgot that the last command failed"
    );
}

/// A quick search owns the info row while it is running, so a message raised
/// underneath it has not been seen. Expiring it there would throw away the
/// result of an F5 or a Ctrl+R the user never got to read.
#[test]
fn a_message_raised_under_a_quick_search_is_not_thrown_away() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("first.txt"), "").expect("first");
    std::fs::write(temp.path().join("second.txt"), "").expect("second");
    let mut app = App::new().expect("app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    app.quick_search.push('f');
    app.set_status(Status::PanelsRefreshed);
    app.expire_status_on_move();
    app.get_active_panel().next();
    app.expire_status_on_move();

    app.quick_search.clear();
    app.expire_status_on_move();
    assert!(
        app.status_news().is_some(),
        "the refresh result was discarded while the search hid it"
    );
}
