//! Purpose: what every row must say — a `snake_case` name of its own, a
//! command of its own, a summary, a group, and where its key comes from.

use super::*;

fn is_snake_case(name: &str) -> bool {
    let words = name.split('_');
    let word = |word: &str| {
        !word.is_empty()
            && word
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    };
    name.starts_with(|ch: char| ch.is_ascii_lowercase()) && words.into_iter().all(word)
}

#[test]
fn snake_case_is_told_from_the_rest() {
    for name in ["view", "sort_by_size", "view_alternative_full", "f9_menu"] {
        assert!(is_snake_case(name), "{name}");
    }
    for name in [
        "",
        "View",
        "sort-by-size",
        "sort__size",
        "_view",
        "view_",
        "9lives",
        "вид",
    ] {
        assert!(!is_snake_case(name), "{name}");
    }
}

#[test]
fn every_action_has_a_snake_case_name_of_its_own() {
    for screen in screens() {
        let names: Vec<&str> = screen.rows.iter().map(|row| row.name.as_str()).collect();
        for name in &names {
            assert!(is_snake_case(name), "{}.{name}", screen.context);
        }
        assert_eq!(repeated(names), Vec::<&str>::new(), "{}", screen.context);
    }
}

/// With the names unique as well, a name and a command stand for each
/// other: the file can say a command and the report can name it.
#[test]
fn every_action_has_a_command_of_its_own() {
    for screen in screens() {
        let commands: Vec<&str> = screen.rows.iter().map(|row| row.command.as_str()).collect();
        assert_eq!(
            repeated(commands),
            Vec::<&str>::new(),
            "{}: two names run one command",
            screen.context
        );
    }
}

#[test]
fn every_row_says_what_it_does_and_every_group_has_a_title() {
    for screen in screens() {
        for row in &screen.rows {
            let action = format!("{}.{}", screen.context, row.name);
            assert_ne!(row.summary.trim().len(), 0, "{action} has no summary");
            assert_eq!(row.summary.trim(), row.summary, "{action}");
            assert!(!row.summary.ends_with('.'), "{action}: a summary is a line");
            assert_ne!(row.group.trim().len(), 0, "{action} is in no group");
        }
    }
}

#[test]
fn an_origin_that_names_a_key_names_one() {
    for screen in screens() {
        for row in &screen.rows {
            let action = format!("{}.{}", screen.context, row.name);
            match row.origin {
                Origin::Far(key) | Origin::Htop(key) | Origin::Bash(key) => {
                    assert_ne!(key.trim().len(), 0, "{action}");
                }
                Origin::FarSame => assert_ne!(
                    row.defaults.len(),
                    0,
                    "{action}: Far's key is the default, and there is none"
                ),
                Origin::FarNoKey => assert_eq!(
                    row.defaults.len(),
                    0,
                    "{action}: Far gives it no key, and it has one"
                ),
                Origin::Ffm => {}
            }
        }
    }
}

#[test]
fn an_action_name_reads_as_its_text() {
    let name = ActionName::new("sort_by_size");
    assert_eq!(name.as_str(), "sort_by_size");
    assert_eq!(name.to_string(), "sort_by_size");
    assert!(ActionName::new("copy") < ActionName::new("move"));
}

#[test]
fn a_row_starts_as_a_command_of_this_programs_and_is_completed_by_its_note() {
    let started = row("view", 3_u8, &[]);
    assert_eq!(started.name, ActionName::new("view"));
    assert_eq!(started.command, 3);
    assert_eq!(started.origin, Origin::Ffm);
    assert_eq!(started.summary, "");
    assert_eq!(started.role, Role::Command);
    let noted = started.note(Origin::Far("F3"), "View the file");
    assert_eq!(noted.origin, Origin::Far("F3"));
    assert_eq!(noted.summary, "View the file");
    assert_eq!(noted.role, Role::Command);
    assert_eq!(noted.exit().role, Role::Exit);
    assert_eq!(noted.exit().summary, "View the file");
}
