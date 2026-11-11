//! Purpose: Settings, file, archive, link and describe rows open their own prompts.

use super::*;

#[test]
fn settings_rows_open_apply_and_persist_their_documented_defaults() {
    let (_temp, mut app) = fixture();
    for entry in Menu::open(Heading::Options).entries() {
        let Some(Action::Menu(MenuCommand::Settings(page))) = entry.action() else {
            continue;
        };
        execute(&mut app, MenuCommand::Settings(page));
        assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
        let value = crate::models::settings::specification(page).1;
        submit(&mut app, MenuPrompt::Settings(page), value).unwrap();
        assert_eq!(app.settings.get(page), value);
        assert!(!app.overlay.is_active());
    }
    execute_result(&mut app, MenuCommand::SaveSetup).unwrap();
    assert!(app.settings.persistence.file().unwrap().is_file());
}

#[test]
fn file_and_archive_rows_open_their_own_prompts() {
    let (_temp, mut app) = fixture();
    execute_tool(&mut app, ToolCommand::FindFile).unwrap();
    assert!(
        matches!(app.overlay, Overlay::FindFile(_)),
        "Find file has its own overlay"
    );

    execute_tool(&mut app, ToolCommand::FindFolder).unwrap();
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("prompt")
    };
    let MenuDialog::Prompt {
        purpose: actual, ..
    } = dialog.as_ref()
    else {
        panic!("prompt")
    };
    assert_eq!(*actual, MenuPrompt::FindFolder);
    execute_tool(&mut app, ToolCommand::Filter).unwrap();
    assert!(matches!(app.overlay, Overlay::Filter(_)));
}

/// Alt+F6 and the Files menu row are one path: the Link name prompt, which
/// asks the kind as well as the name, not the generic text prompt.
#[test]
fn the_link_row_opens_the_link_name_prompt() {
    let (_temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::Link);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the Link name prompt")
    };
    assert_eq!(prompt.purpose(), crate::models::NamePurpose::Link);
}

/// The Describe row has a dialog of its own rather than the generic prompt:
/// one per marked entry, opening on the current text.
#[test]
fn the_describe_row_opens_its_own_dialog_or_says_there_is_nothing() {
    let (_temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::Describe);
    let Overlay::Describe(prompt) = &app.overlay else {
        panic!("the describe dialog is open")
    };
    assert_eq!(prompt.name(), "alpha.txt");

    app.close_overlay();
    app.left_panel.select(None);
    execute_file(&mut app, FileCommand::Describe);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status,
        Status::CommandOutput("Nothing to describe".to_owned())
    );
}
