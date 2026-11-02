//! Purpose: Named automation uses the real modal dispatcher and preserves custom bindings.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::{
    automation::catalog,
    models::{Editor, Overlay, Panel},
    WritableTestBackend,
};

mod contexts;
mod interpreters;
mod management;
mod panels;

struct Fixture {
    app: App,
    terminal: Terminal<WritableTestBackend>,
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        for side in ["left", "right"] {
            fs::create_dir(root.path().join(side)).unwrap();
        }
        fs::write(root.path().join("left/file.txt"), "alpha").unwrap();
        let mut left = Panel::new(root.path().join("left")).unwrap();
        let at = left
            .entries
            .iter()
            .position(|entry| entry.name.as_str() == "file.txt")
            .unwrap();
        left.select(Some(at));
        let right = Panel::new(root.path().join("right")).unwrap();
        Self {
            app: App::with_panels(left, right),
            terminal: Terminal::new(WritableTestBackend::new(100, 30)).unwrap(),
            root,
        }
    }

    fn run(&mut self, context: &str, action: &str) -> io::Result<()> {
        invoke(
            &mut self.terminal,
            &mut self.app,
            catalog::resolve(context, action).unwrap(),
        )
    }

    fn editor(&mut self) {
        self.app.overlay = Overlay::Editor(Editor::from_content(
            "file.txt".into(),
            self.root.path().join("left/file.txt"),
            "alpha",
        ));
    }
}

#[test]
fn copy_opens_the_real_prompt_without_writing_the_destination() {
    let mut fixture = Fixture::new();
    fixture.run("panels", "copy").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::CopyPrompt(_)));
    assert!(!fixture.root.path().join("right/file.txt").exists());
    assert_eq!(current_context(&fixture.app), None);
    assert_eq!(
        fixture.run("panels", "copy").unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(!fixture.root.path().join("right/file.txt").exists());
}

#[test]
fn editor_commands_ignore_user_bindings_and_restore_the_whole_keymap() {
    let mut fixture = Fixture::new();
    fixture.editor();
    fixture.app.keymap.editor = crate::action::Bindings::default();
    fixture.app.keymap.editor.bind(
        crate::action::editor::EditorCommand::Close,
        "F24".parse().unwrap(),
    );
    let original = fixture.app.keymap.clone();
    fixture.run("editor", "newline").unwrap();
    assert_eq!(fixture.app.get_editor().unwrap().content(), "\nalpha");
    assert_eq!(fixture.app.keymap, original);
    fixture.run("editor", "save").unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("left/file.txt")).unwrap(),
        "\nalpha"
    );
    assert_eq!(fixture.app.keymap, original);
}

#[test]
fn wrong_context_does_not_change_the_buffer_or_any_binding() {
    let mut fixture = Fixture::new();
    fixture.editor();
    let original = fixture.app.keymap.clone();
    assert_eq!(
        fixture.run("panels", "copy").unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(fixture.app.get_editor().unwrap().content(), "alpha");
    assert_eq!(fixture.app.keymap, original);
    fixture.app.overlay = Overlay::None;
    assert!(fixture.run("editor", "save").is_err());
}
