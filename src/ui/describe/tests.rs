//! Unit tests: the Describe files dialog names the entry, shows the text
//! being typed, and puts the caret where the next character goes.

use ratatui::Terminal;

use crate::models::{App, DescribePrompt, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

/// The screen and where the terminal cursor ended up. A draw that sets no
/// caret leaves the test backend's cursor where it started, at the origin.
fn draw(app: &mut App) -> (String, (u16, u16)) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal");
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    let cursor = terminal.get_cursor_position().expect("cursor");
    (buffer_text(&terminal), (cursor.x, cursor.y))
}

#[test]
fn the_dialog_names_the_entry_and_shows_the_text() {
    let mut app = App::new().expect("test app");
    let mut prompt = DescribePrompt::new("notes.txt".to_owned(), "Old", Vec::new());
    prompt.push('!');
    app.set_overlay(Overlay::Describe(prompt));
    let (screen, cursor) = draw(&mut app);
    assert!(screen.contains("Describe files"), "{screen}");
    assert!(screen.contains("notes.txt"), "{screen}");
    assert!(screen.contains("Old!"), "{screen}");
    assert!(
        screen.contains("OK") && screen.contains("Cancel"),
        "{screen}"
    );
    assert!(!screen.contains("Coming in this release"), "{screen}");
    assert!(
        !screen.contains('\u{2193}'),
        "no history marker: the field has no history behind it: {screen}"
    );
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");

    let (x, y) = cursor;
    assert_ne!((x, y), (0, 0), "the caret sits in the input");
    let row = screen.lines().nth(usize::from(y)).expect("caret row");
    let before: String = row.chars().take(usize::from(x)).collect();
    assert!(before.ends_with("Old!"), "caret after the text: {before:?}");
}

#[test]
fn focus_on_cancel_takes_the_caret_out_of_the_input() {
    let mut app = App::new().expect("test app");
    let mut prompt = DescribePrompt::new("notes.txt".to_owned(), "", Vec::new());
    prompt.toggle_button();
    app.set_overlay(Overlay::Describe(prompt));
    let (screen, cursor) = draw(&mut app);
    assert_eq!(cursor, (0, 0), "no caret was placed: {screen}");
}

/// Left backs the caret up, and the terminal cursor follows it: the next
/// character goes in front of what stands to the right.
#[test]
fn the_caret_follows_the_prompts_cursor_into_the_text() {
    let mut app = App::new().expect("test app");
    let mut prompt = DescribePrompt::new("notes.txt".to_owned(), "Old!", Vec::new());
    prompt.left();
    prompt.left();
    app.set_overlay(Overlay::Describe(prompt));
    let (screen, (x, y)) = draw(&mut app);
    let row = screen.lines().nth(usize::from(y)).expect("caret row");
    let before: String = row.chars().take(usize::from(x)).collect();
    assert!(before.ends_with("Ol"), "caret before `d!`: {before:?}");
    let after: String = row.chars().skip(usize::from(x)).take(2).collect();
    assert_eq!(after, "d!");
}

/// A name with an ampersand is shown whole: `&` is a hotkey marker in a
/// dialog label, and a file called `a&b.txt` is not called `ab.txt`.
#[test]
fn an_ampersand_in_the_name_stays_in_the_caption() {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Describe(DescribePrompt::new(
        "a&b.txt".to_owned(),
        "",
        Vec::new(),
    )));
    let (screen, _) = draw(&mut app);
    assert!(screen.contains("a&b.txt"), "{screen}");
}
