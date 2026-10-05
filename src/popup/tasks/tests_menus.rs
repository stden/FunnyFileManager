//! Purpose: Tests for the task list's menus: the Sort by menu and its
//! Reverse order row, and Show processes of.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{Modal, Owner, ProcessSample, SortKey, SortOrder, TaskMenu, UserScope};
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};

use super::tests::{chord, press, three};

fn menu_row(tasks: &mut TaskApp) -> usize {
    let Modal::Menu(menu) = tasks.list().modal() else {
        panic!("a menu is open");
    };
    menu.cursor().index()
}

#[test]
fn the_sort_menu_walks_its_rows_and_enter_chooses_one() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('<'));
    assert_eq!(menu_row(&mut tasks), 7, "on CPU%, the key in force");
    press(&mut tasks, KeyCode::Home);
    assert_eq!(menu_row(&mut tasks), 0);
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::PageDown);
    assert_eq!(menu_row(&mut tasks), 11);
    press(&mut tasks, KeyCode::PageDown);
    assert_eq!(menu_row(&mut tasks), TaskMenu::SORT_ROWS - 1, "clamped");
    press(&mut tasks, KeyCode::Up);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.list().options().sort.key(), SortKey::Command);
}

#[test]
fn reverse_order_is_the_last_row_and_capital_i() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(6));
    press(&mut tasks, KeyCode::End);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().options().sort.order(), SortOrder::Reversed);
    chord(&mut tasks, KeyCode::F(12), KeyModifiers::CONTROL);
    chord(&mut tasks, KeyCode::Char('i'), KeyModifiers::SHIFT);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.list().options().sort.order(), SortOrder::Direct);
    press(&mut tasks, KeyCode::Char('u'));
    press(&mut tasks, KeyCode::Char('I'));
    assert!(
        matches!(tasks.list().modal(), Modal::Menu(TaskMenu::User { .. })),
        "I only reverses from Sort by"
    );
}

#[test]
fn esc_and_f10_leave_a_menu_without_choosing() {
    for code in [KeyCode::Esc, KeyCode::F(10)] {
        let mut tasks = three();
        press(&mut tasks, KeyCode::Char('>'));
        press(&mut tasks, KeyCode::Home);
        press(&mut tasks, code);
        assert_eq!(tasks.list().modal(), &Modal::Browse);
        assert_eq!(tasks.list().options().sort.key(), SortKey::Cpu);
        assert!(tasks.app.overlay.is_active());
    }
}

#[test]
fn show_processes_of_scopes_the_list_to_one_owner_or_all() {
    let alice = ProcessSample {
        owner: Owner::Named("alice".into()),
        ..named(40, "sshd")
    };
    let mut tasks = TaskApp::open(vec![named(10, "vim"), alice], Privilege::User);
    press(&mut tasks, KeyCode::Char('u'));
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(
        tasks.list().options().scope,
        UserScope::Only(Owner::Named("alice".into()))
    );
    assert_eq!(tasks.shown(), [40]);
    press(&mut tasks, KeyCode::Char('u'));
    assert_eq!(menu_row(&mut tasks), 1, "on the scope in force");
    press(&mut tasks, KeyCode::Up);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().options().scope, UserScope::All);
    assert_eq!(tasks.shown(), [10, 40]);
}
