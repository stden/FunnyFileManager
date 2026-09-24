//! Purpose: the File and Tool families, and the other actions that open a dialog or report.

use super::*;

#[test]
fn each_file_command_opens_its_own_dialog() {
    let cases: [(FileCommand, Opens); 8] = [
        (FileCommand::Link, |o| matches!(o, Overlay::NamePrompt(_))),
        (FileCommand::Wipe, |o| matches!(o, Overlay::WipeConfirm(_))),
        (FileCommand::AddToArchive, |o| {
            matches!(o, Overlay::ArchivePrompt(_))
        }),
        (FileCommand::Extract, |o| {
            matches!(o, Overlay::ExtractPrompt(_))
        }),
        (FileCommand::ArchiveCommands, |o| {
            matches!(o, Overlay::ArchiveCommands(_))
        }),
        (FileCommand::Attributes, |o| {
            matches!(o, Overlay::Attributes(_))
        }),
        (FileCommand::ApplyCommand, |o| {
            matches!(o, Overlay::ApplyCommand(_))
        }),
        (FileCommand::Describe, |o| matches!(o, Overlay::Describe(_))),
    ];
    for (command, is_open) in cases {
        let mut app = App::new().expect("test app");
        // Off `..`: a file command has nothing to ask about there, and
        // Wipe, like F8, declines rather than opening a dialog about nothing.
        assert!(app.get_active_panel().select_named("Cargo.toml"));
        apply_action(&mut terminal(), &mut app, Action::File(command));
        assert!(is_open(&app.overlay), "{command:?}");
    }
}

#[test]
fn each_tool_opens_its_own_dialog() {
    let cases: [(ToolCommand, Opens); 8] = [
        (ToolCommand::FindFile, |o| matches!(o, Overlay::FindFile(_))),
        (ToolCommand::History, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (ToolCommand::FindFolder, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (ToolCommand::ViewHistory, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (ToolCommand::FolderHistory, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (ToolCommand::Filter, |o| matches!(o, Overlay::MenuDialog(_))),
        (ToolCommand::Screens, |o| {
            matches!(o, Overlay::MenuDialog(_))
        }),
        (ToolCommand::Tasks, |o| matches!(o, Overlay::Tasks(_))),
    ];
    for (command, is_open) in cases {
        let mut app = App::new().expect("test app");
        let host = crate::test_support::processes::FixtureHost::new(Vec::new());
        app.task_host.install(std::sync::Arc::new(host));
        apply_action(&mut terminal(), &mut app, Action::Tool(command));
        assert!(is_open(&app.overlay), "{command:?}");
    }
}

/// Alt+F9 runs the same terminal resize as the F9 row. The resize itself is
/// skipped here so the test does not change the host terminal.
#[test]
fn video_mode_uses_the_terminal_toggle() {
    let _guard = crate::terminal::skip_video_mode_resize();
    let before = crate::terminal::video_mode_calls();
    let mut app = App::new().expect("test app");
    apply_action(
        &mut terminal(),
        &mut app,
        Action::Tool(ToolCommand::VideoMode),
    );
    assert!(!app.overlay.is_active());
    assert_eq!(crate::terminal::video_mode_calls(), before + 1);
    assert!(
        !app.status.to_string().contains("coming in this release"),
        "{}",
        app.status
    );
}

/// Alt+F9 must never be silent: the terminal may ignore the resize (tmux
/// does), so the status bar says what was asked for and what the terminal
/// has. The skipped resize stands for a terminal that kept its height.
#[test]
fn alt_f9_reports_what_the_terminal_did_with_the_resize() {
    let _guard = crate::terminal::skip_video_mode_resize();
    let mut app = App::new().expect("test app");
    let status_before = app.status.to_string();
    dispatch_key(
        &mut terminal(),
        &mut app,
        KeyEvent::new(KeyCode::F(9), KeyModifiers::ALT),
    );
    let status = app.status.to_string();
    assert_ne!(status, status_before, "Alt+F9 left the status unchanged");
    assert!(status.contains("Video mode"), "{status}");
}

/// F12 lists the screens from the console too, where every other panel
/// action is swallowed.
#[test]
fn f12_lists_the_screens_from_the_console() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    app.view = ViewMode::Console;
    apply_action(&mut terminal, &mut app, Action::Tool(ToolCommand::Screens));
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
    app.close_overlay();
    apply_action(&mut terminal, &mut app, Action::Tool(ToolCommand::FindFile));
    assert!(!app.overlay.is_active(), "other tools stay panel actions");
}

/// Draw the open dialog, then click its `index`-th button where it landed.
fn click_dialog_button(terminal: &mut Terminal<WritableTestBackend>, app: &mut App, index: usize) {
    terminal.draw(|f| ui(f, app)).expect("draw");
    let &(_, left, _) = app
        .dialog_mouse
        .buttons
        .iter()
        .find(|(button, _, _)| *button == index)
        .expect("the button was drawn");
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: left,
        row: app.dialog_mouse.buttons_y,
        modifiers: KeyModifiers::NONE,
    };
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    let mut clicks = crate::mouse::PanelClicks::default();
    dispatch_mouse(terminal, app, mouse, screen, &mut clicks);
}

/// A click on an archive dialog's button names it before Enter, so the
/// Extract / Add button acts as itself and Cancel cancels, whatever the
/// keyboard focus was.
#[test]
fn archive_dialog_buttons_answer_to_the_mouse() {
    let mut terminal = terminal();
    let mut app = App::new().expect("test app");
    ops::archive::open_extract(&mut app);
    click_dialog_button(&mut terminal, &mut app, 0);
    assert!(
        !app.overlay.is_active(),
        "Extract with nothing to extract reports"
    );
    assert!(
        matches!(&app.status, status::Status::CommandOutput(text) if text.contains("No archive")),
        "{:?}",
        app.status
    );

    assert!(app.get_active_panel().select_named("Cargo.toml"));
    ops::archive::open_add(&mut app);
    click_dialog_button(&mut terminal, &mut app, 1);
    assert!(!app.overlay.is_active(), "Cancel cancels");
    assert!(!app
        .get_inactive_panel_ref()
        .path
        .join("Cargo.tar.gz")
        .exists());
}

#[test]
fn wsl_management_opens_the_manager() {
    let mut app = App::new().expect("test app");
    apply_action(&mut terminal(), &mut app, Action::WslManagement);
    assert!(matches!(app.overlay, Overlay::Wsl(_)));
}

/// Shift+F9 is the menu's Save setup, not a second route to another writer:
/// it writes the page values and keeps every key it does not own.
#[test]
fn shift_f9_is_the_menu_save_setup_and_keeps_the_keys_it_does_not_own() {
    let shift_f9 = KeyEvent::new(KeyCode::F(9), KeyModifiers::SHIFT);
    let action = resolve(shift_f9);
    assert_eq!(action, Action::MenuCommand(MenuCommand::SaveSetup));

    let temp = tempfile::TempDir::new().expect("dir");
    let path = temp.path().join("settings.yaml");
    std::fs::write(
        &path,
        "lsp:\n  python:\n    enabled: false\noptions:\n  System: '5000'\nunknown: 1\n",
    )
    .expect("fixture");
    let mut app = App::new().expect("app");
    app.settings = models::settings::Settings::with_file(path.clone());
    app.settings.set(SettingsPage::Editor, "8").expect("valid");
    apply_action(&mut terminal(), &mut app, action);

    let yaml = |text: &str| -> serde_norway::Value { serde_norway::from_str(text).expect("YAML") };
    assert_eq!(
        yaml(&std::fs::read_to_string(&path).expect("saved")),
        yaml("lsp:\n  python:\n    enabled: false\noptions:\n  System: '5000'\n  Editor: '8'\nunknown: 1\n")
    );
    assert_eq!(app.status, status::Status::SetupSaved(path));
}

#[test]
fn a_configured_mask_is_the_group_selection_suggestion() {
    let mut app = App::new().expect("test app");
    app.settings.set(SettingsPage::Masks, "*.rs").expect("mask");
    apply_action(&mut terminal(), &mut app, Action::SelectGroup);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("the mask prompt opened");
    };
    assert_eq!(prompt.mask(), "*.rs");
}
