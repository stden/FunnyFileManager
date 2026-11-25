//! Purpose: a click past a dialog's last control leaves its focus where it was.

use super::*;

#[test]
fn clicking_below_the_mcp_client_list_keeps_its_focus() {
    use crate::mcp_clients::{Scope, ServerState, Survey};
    use crate::models::{McpInstallDialog, McpInstallField};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let survey = Survey {
        server: ServerState::Missing,
        folder: None,
        scope: Scope::AllProjects,
        clients: Vec::new(),
    };
    app.set_overlay(Overlay::McpInstall(Box::new(McpInstallDialog::new(survey))));
    click_dialog_row(&mut app, 999);
    assert!(matches!(&app.overlay, Overlay::McpInstall(d) if d.field == McpInstallField::Scope));
}

#[test]
fn disabled_mcp_client_row_is_ignored() {
    use crate::mcp_clients::{
        ClientId, ClientStatus, ConfigState, Presence, Scope, ServerState, Survey,
    };
    use crate::models::{McpInstallDialog, McpInstallField};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let survey = Survey {
        server: ServerState::Missing,
        folder: None,
        scope: Scope::AllProjects,
        clients: vec![ClientStatus {
            client: ClientId::ClaudeCode,
            presence: Presence::Detected,
            config: None,
            state: ConfigState::NotConfigured,
        }],
    };
    let n = crate::mcp_clients::server_lines(&survey.server).len();
    app.set_overlay(Overlay::McpInstall(Box::new(McpInstallDialog::new(survey))));
    click_dialog_row(&mut app, n + 4);
    assert!(matches!(&app.overlay, Overlay::McpInstall(d) if d.field == McpInstallField::Scope));
}

#[test]
fn clicking_button_without_dialog_key_does_nothing() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Help);
    app.dialog_mouse.area = Some(ratatui::layout::Rect::new(0, 0, 20, 10));
    app.dialog_mouse.buttons_y = 8;
    app.dialog_mouse.buttons = vec![(0, 2, 6)];
    let mut term = terminal();
    gesture(
        &mut term,
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        (3, 8),
    );
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn non_checkbox_copy_row_and_focused_input_button_preserve_state() {
    use crate::models::{InputPrompt, InputPurpose};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::new(
        "a.txt",
        Path::new("/tmp"),
    ))));
    click_dialog_row(&mut app, 6);
    click_dialog_row(&mut app, 1);
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("copy dialog");
    };
    assert_eq!(prompt.field, CopyField::Destination);

    let prompt = InputPrompt::new(InputPurpose::ImportNetBox, "");
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(matches!(&app.overlay, Overlay::InputPrompt(p) if !p.on_cancel()));
}

#[test]
fn out_of_range_encoding_and_non_search_editor_modal_rows_do_nothing() {
    use crate::encoding::TextEncoding;
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut editor = Editor::from_content("text".into(), dir.path().join("text"), "hello");
    editor.encoding_choice = Some(TextEncoding::Utf8);
    app.set_overlay(Overlay::Editor(editor));
    click_dialog_row(&mut app, 99);
    assert_eq!(
        app.get_editor().expect("editor").encoding_choice,
        Some(TextEncoding::Utf8)
    );

    app.set_overlay(Overlay::Editor(Editor {
        state: EditState::ConfirmingExit(ExitChoice::Save),
        ..Default::default()
    }));
    click_dialog_row(&mut app, 1);
    assert!(matches!(
        app.get_editor().expect("editor").state,
        EditState::ConfirmingExit(ExitChoice::Save)
    ));
}

#[test]
fn empty_attributes_dialog_and_group_offset_cover_all_branches() {
    use crate::models::{AttributesDialog, AttributesField, Flag, FlagGroup};
    let groups = [
        FlagGroup {
            caption: "A",
            flags: &[Flag::OwnerRead, Flag::OwnerWrite],
        },
        FlagGroup {
            caption: "B",
            flags: &[Flag::OwnerExecute],
        },
    ];
    assert_eq!(group_start_flag(&groups, 1), 2);

    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Attributes(Box::<AttributesDialog>::default()));
    click_dialog_row(&mut app, 3);
    click_dialog_row(&mut app, 99);
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("attributes");
    };
    assert_eq!(dialog.field, AttributesField::Flag(0));
}
