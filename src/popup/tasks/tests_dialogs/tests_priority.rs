//! Purpose: the task list's priority form (F7) and the `[` / `]` keys — stepping and typing a nice value, cancelling, and who is asked before lowering it.

use super::*;

#[test]
fn the_priority_form_steps_within_the_range_and_sets_on_enter() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(7));
    for _ in 0..30 {
        press(&mut tasks, KeyCode::Up);
    }
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), reniced(-19));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

#[test]
fn the_priority_form_edits_its_field_and_set_waits_for_a_nice_value() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(7));
    press(&mut tasks, KeyCode::Backspace);
    type_text(&mut tasks, "33");
    press(&mut tasks, KeyCode::Enter);
    assert!(
        matches!(tasks.list().modal(), Modal::Priority(_)),
        "disabled"
    );
    press(&mut tasks, KeyCode::Backspace);
    press(&mut tasks, KeyCode::Tab);
    press(&mut tasks, KeyCode::Char('9'));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), reniced(3), "Set, not typed into");
}

#[test]
fn the_priority_forms_cancel_and_escape_set_nothing() {
    for keys in [
        vec![KeyCode::Tab, KeyCode::Tab, KeyCode::Enter],
        vec![KeyCode::BackTab, KeyCode::Enter],
        vec![KeyCode::F(10)],
        vec![KeyCode::Esc],
    ] {
        let mut tasks = firefox(Privilege::User);
        press(&mut tasks, KeyCode::F(7));
        for code in &keys {
            press(&mut tasks, *code);
        }
        assert!(tasks.host.controls().is_empty(), "{keys:?}");
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{keys:?}");
    }
}

#[test]
fn square_brackets_step_the_nice_value_and_a_user_confirms_lowering() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::Char(']'));
    press(&mut tasks, KeyCode::Char(']'));
    assert_eq!(
        tasks.host.controls(),
        [reniced(4), reniced(3)].concat(),
        "the second step starts from the accepted value"
    );
    press(&mut tasks, KeyCode::Char('['));
    assert!(matches!(tasks.list().modal(), Modal::Renice(_)));
    press(&mut tasks, KeyCode::Tab);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls().len(), 2, "Cancel was focused");
    press(&mut tasks, KeyCode::Char('['));
    press(&mut tasks, KeyCode::Char('x'));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls().last(), reniced(4).last());
    press(&mut tasks, KeyCode::Char('['));
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.host.controls().len(), 3);
}

#[test]
fn root_lowers_the_priority_at_once() {
    let mut tasks = firefox(Privilege::Root);
    press(&mut tasks, KeyCode::Char('['));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.host.controls(), reniced(6));
}
