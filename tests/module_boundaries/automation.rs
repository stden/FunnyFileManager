//! Purpose: Named automation executes typed commands without synthesizing keyboard input.

use super::architecture::{forbidden_dependencies, SourceFile};

fn keyboard_dependencies(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| {
        !file.is_test()
            && file.in_module("application")
            && file.module.get(1).is_some_and(|name| name == "automation")
    };
    super::messages(&forbidden_dependencies(
        sources,
        checked,
        &[
            "crossterm::event",
            "crate::action::KeyChord",
            "crate::action::Bindings",
        ],
    ))
}

#[test]
fn named_automation_executes_commands_without_keyboard_dependencies() {
    assert_eq!(
        keyboard_dependencies(&super::tui_sources()),
        Vec::<String>::new()
    );
}

#[test]
fn named_automation_rejects_synthetic_input_and_temporary_bindings() {
    let sources = [SourceFile::new(
        "src/application/automation/inject.rs",
        "use crossterm::event::{KeyCode, KeyEvent};\nuse crate::action::{Bindings, KeyChord};\n",
    )];
    assert_eq!(
        keyboard_dependencies(&sources),
        [
            "src/application/automation/inject.rs:1: `crossterm::event::KeyCode` (forbidden: `crossterm::event`)",
            "src/application/automation/inject.rs:1: `crossterm::event::KeyEvent` (forbidden: `crossterm::event`)",
            "src/application/automation/inject.rs:2: `crate::action::Bindings` (forbidden: `crate::action::Bindings`)",
            "src/application/automation/inject.rs:2: `crate::action::KeyChord` (forbidden: `crate::action::KeyChord`)",
        ]
    );
}

#[test]
fn named_automation_allows_typed_commands_and_raw_input_in_its_own_owner() {
    let sources = [
        SourceFile::new(
            "src/application/automation.rs",
            "use crate::action::editor::EditorCommand;\nuse crate::popup;\n",
        ),
        SourceFile::new(
            "src/application/event.rs",
            "use crossterm::event::KeyEvent;\n",
        ),
        SourceFile::new(
            "src/application/automation/tests.rs",
            "use crossterm::event::KeyEvent;\n",
        ),
    ];
    assert_eq!(keyboard_dependencies(&sources), Vec::<String>::new());
}
