//! Purpose: the task list's mask dialog marking matching names, and its help page (F1) scrolling and closing.

use super::*;
use crate::action::tasks::HELP;

#[test]
fn the_mask_dialog_marks_the_matching_names_and_says_how_many() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('+'));
    type_text(&mut tasks, "*n*");
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Marked {
            changed: 2,
            total: 2
        })
    );
    press(&mut tasks, KeyCode::Char('-'));
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.list().marked_count(), 2, "cancelled");
}

#[test]
fn the_help_page_scrolls_within_its_rows_and_closes_on_its_keys() {
    let mut tasks = three();
    let viewport = crate::models::Viewport::new(12);
    tasks.app.viewport = viewport;
    let page = tasks.list().page_for(viewport);
    let last = HELP.len() - page;
    let top = |tasks: &mut TaskApp| match tasks.list().modal() {
        Modal::Help(top) => top.lines(),
        other => panic!("the help page is open, not {other:?}"),
    };
    press(&mut tasks, KeyCode::F(1));
    press(&mut tasks, KeyCode::End);
    assert_eq!(top(&mut tasks), last);
    press(&mut tasks, KeyCode::Down);
    assert_eq!(top(&mut tasks), last, "no further");
    press(&mut tasks, KeyCode::PageUp);
    press(&mut tasks, KeyCode::Up);
    assert_eq!(top(&mut tasks), last - page - 1);
    press(&mut tasks, KeyCode::PageDown);
    press(&mut tasks, KeyCode::Home);
    assert_eq!(top(&mut tasks), 0);
    press(&mut tasks, KeyCode::Char('x'));
    assert_eq!(top(&mut tasks), 0);
}

#[test]
fn the_help_page_closes_on_its_keys_and_only_the_page_closes() {
    let mut tasks = three();
    for close in ['q', 'h', '?'] {
        press(&mut tasks, KeyCode::F(1));
        press(&mut tasks, KeyCode::Char(close));
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{close}");
    }
    chord(&mut tasks, KeyCode::Char('?'), KeyModifiers::SHIFT);
    press(&mut tasks, KeyCode::F(1));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    press(&mut tasks, KeyCode::Char('h'));
    press(&mut tasks, KeyCode::F(10));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert!(tasks.app.overlay.is_active(), "F10 closed only the help");
}
