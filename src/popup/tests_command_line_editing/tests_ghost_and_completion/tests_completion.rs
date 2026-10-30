//! Purpose: Tab completion replaces the token at the caret and preserves its tail.

use super::*;

#[test]
#[expect(
    clippy::string_slice,
    reason = "a caret off a char boundary panics the test, which fails it as loudly as the assertion"
)]
fn tab_completes_the_token_under_the_caret_and_keeps_the_tail() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("Documents")).expect("mkdir");

    let mut app = prompt("cat Doc /etc/hosts");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;

    // Park the caret right after "Doc", with the second argument to its right.
    send(&mut app, &[ctrl('a')]);
    for _ in 0.."cat Doc".len() {
        send(&mut app, &[key(KeyCode::Right)]);
    }
    send(&mut app, &[key(KeyCode::Tab)]);

    let (text, cursor) = line(&app);
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(text, format!("cat Documents{sep} /etc/hosts"));
    assert_eq!(
        &text[..cursor],
        format!("cat Documents{sep}"),
        "the caret stays at the end of what was just completed"
    );
}
