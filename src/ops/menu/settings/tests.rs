use super::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};
use std::fs;

fn app_in(temp: &tempfile::TempDir) -> io::Result<App> {
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    Ok(App::from_panels(panel.clone(), panel))
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

#[test]
fn load_default_uses_the_settings_file_under_the_config_home() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let path = temp.path().join("ffm").join("settings.yaml");
    write_file(&path, "options:\n  Editor: '8'\n")?;
    let mut app = app_in(&temp)?;
    load_default(&mut app);
    assert_eq!(app.settings.persistence.file(), Some(path.as_path()));
    assert_eq!(app.settings.number(SettingsPage::Editor), 8);
    assert!(!matches!(app.status, Status::Error(_)), "{}", app.status);
    Ok(())
}

/// The failed file stays the save target, so Save setup can say why it will
/// not overwrite the file instead of claiming there is none.
#[test]
fn a_settings_file_that_fails_to_load_is_reported_and_never_overwritten() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("ffm").join("settings.yaml");
    write_file(&path, "Editor: [\n")?;
    let mut app = app_in(&temp)?;
    restore(&mut app, path.clone());
    let status = app.status.to_string();
    assert!(
        status.contains("Settings: settings.yaml does not parse"),
        "{status}"
    );
    assert_eq!(app.settings.persistence.file(), Some(path.as_path()));
    let error = save(&mut app).expect_err("the broken file is not replaced");
    let message = error.to_string();
    assert!(
        message.starts_with("Not saved: settings.yaml does not parse"),
        "{message}"
    );
    assert_eq!(fs::read_to_string(&path)?, "Editor: [\n");
    Ok(())
}

#[test]
fn unavailable_language_rolls_back_loaded_panel_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    fs::write(temp.path().join(".hidden"), "keep visible")?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    let path = temp.path().join("settings.yaml");
    let values = BTreeMap::from([
        (SettingsPage::Panel, "off".to_owned()),
        (
            SettingsPage::Language,
            temp.path().join("missing.json").display().to_string(),
        ),
    ]);
    let section = OptionsSection {
        options: PageValues(values),
    };
    fs::write(
        &path,
        serde_norway::to_string(&section).map_err(io::Error::other)?,
    )?;
    assert!(load(&mut app, &path).is_err());
    assert!(app.settings.enabled(SettingsPage::Panel));
    assert!(!app.left_panel.options.hide_hidden);
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == ".hidden"));
    Ok(())
}

#[test]
fn saved_setup_restores_validated_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("config/settings.yaml");
    let mut app = App::new()?;
    load(&mut app, &path)?;
    app.settings.set(SettingsPage::Editor, "8").unwrap();
    save(&mut app)?;
    assert!(fs::read_to_string(&path)?.contains("\noptions:\n  Editor: '8'\n"));
    let mut restored = App::new()?;
    load(&mut restored, &path)?;
    assert_eq!(restored.settings.number(SettingsPage::Editor), 8);
    assert!(restored.settings.enabled(SettingsPage::Panel));
    Ok(())
}

#[test]
fn invalid_setup_does_not_replace_the_current_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '0'}\n")?;
    let mut app = App::new()?;
    let error = load(&mut app, &path).expect_err("0 is out of range");
    assert!(
        error.to_string().starts_with("Editor settings: "),
        "{error}"
    );
    assert_eq!(app.settings.number(SettingsPage::Editor), 4);
    Ok(())
}

#[test]
fn the_options_pages_read_the_legacy_json_until_there_is_yaml() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(temp.path().join("settings.json"), r#"{"Editor":"8"}"#)?;
    let mut app = App::new()?;
    load(&mut app, &path)?;
    assert_eq!(app.settings.number(SettingsPage::Editor), 8);
    fs::write(&path, "options: {Editor: 2}\n")?;
    load(&mut app, &path)?;
    assert_eq!(app.settings.number(SettingsPage::Editor), 2, "YAML wins");
    Ok(())
}

/// Save setup used to write only the page map, erasing the language servers
/// and every typed page; the first save also migrates the legacy JSON.
#[test]
fn save_setup_keeps_the_typed_settings_and_leaves_the_legacy_json_alone() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    let legacy = temp.path().join("settings.json");
    let json = r#"{"lsp":{"python":{"enabled":false}},"system":{"history_size":5},"Panel":"off"}"#;
    fs::write(&legacy, json)?;
    let mut app = App::new()?;
    load(&mut app, &path)?;
    app.settings.set(SettingsPage::Editor, "8").unwrap();
    save(&mut app)?;
    assert_eq!(fs::read_to_string(&legacy)?, json);
    let stored = document::read(&path)?.expect("settings.yaml");
    let typed: crate::config::Settings = stored.parse()?;
    assert!(!typed.lsp.0["python"].enabled);
    assert_eq!(typed.system.history_size, 5);
    let OptionsSection {
        options: PageValues(pages),
    } = stored.parse()?;
    assert_eq!(pages[&SettingsPage::Panel], "off");
    assert_eq!(pages[&SettingsPage::Editor], "8");
    Ok(())
}

#[test]
fn translation_dictionary_loads_and_invalid_replacements_preserve_it() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    let path = temp.path().join("language.json");
    fs::write(&path, r#"{"&Brief":"&Compact"}"#)?;
    submit(
        &mut app,
        SettingsPage::Language,
        &path.display().to_string(),
    )?;
    assert_eq!(app.settings.translate("&Brief"), "&Compact");
    fs::write(&path, "not JSON")?;
    assert!(submit(
        &mut app,
        SettingsPage::Language,
        &path.display().to_string()
    )
    .is_err());
    assert_eq!(app.settings.translate("&Brief"), "&Compact");
    let large = fs::File::create(&path)?;
    large.set_len(1_048_577)?;
    assert!(translations(&path.display().to_string()).is_err());
    assert!(load(&mut app, &path).is_err());
    Ok(())
}

#[test]
fn changing_description_filename_refreshes_the_current_description_view() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    fs::write(temp.path().join("item"), "body")?;
    fs::write(temp.path().join("notes"), "item New description\n")?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    app.left_panel.view = crate::models::PanelView::Descriptions;
    submit(&mut app, SettingsPage::Descriptions, "notes")?;
    assert_eq!(
        app.left_panel.details.descriptions["item"],
        "New description"
    );
    Ok(())
}

#[test]
fn panel_modes_apply_to_both_listings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    submit(&mut app, SettingsPage::PanelModes, "Wide")?;
    assert_eq!(app.left_panel.view, crate::models::PanelView::Wide);
    assert_eq!(app.right_panel.view, crate::models::PanelView::Wide);
    Ok(())
}

#[test]
fn folder_description_settings_reload_the_quick_view() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    submit(&mut app, SettingsPage::FolderDescriptions, "README")?;
    submit(&mut app, SettingsPage::Information, "off")?;
    assert!(!app.settings.enabled(SettingsPage::Information));
    Ok(())
}

#[test]
fn save_without_a_file_is_an_error() {
    let mut app = App::new().expect("app");
    app.settings.persistence = Persistence::SessionOnly;
    assert!(save(&mut app).is_err());
}

#[test]
fn open_puts_the_current_value_in_a_prompt() {
    let mut app = App::new().expect("app");
    open(&mut app, SettingsPage::Editor);
    assert!(matches!(app.overlay, crate::models::Overlay::MenuDialog(_)));
}

/// Opening a FIFO blocks until something writes to it, and this runs on the
/// event loop: the whole UI froze. A dictionary or settings file that is not
/// a regular file is refused before it is opened.
#[cfg(unix)]
#[test]
fn a_fifo_is_refused_as_a_language_or_settings_file_without_blocking() {
    let temp = tempfile::TempDir::new().expect("dir");
    let fifo = temp.path().join("fifo");
    crate::test_support::make_fifo(&fifo).expect("mkfifo");
    let language = fifo.display().to_string();
    let refused =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            translations(&language).map_err(|error| error.to_string())
        })
        .expect("Options > Languages must not block on a FIFO");
    let error = refused.expect_err("a FIFO is not a dictionary");
    assert!(error.contains("not a regular file"), "{error}");

    let settings =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            let panel = crate::models::Panel::from_entries(PathBuf::from("/"), Vec::new());
            let mut app = App::with_panels(panel.clone(), panel);
            load(&mut app, &fifo).map_err(|error| error.to_string())
        })
        .expect("loading settings must not block on a FIFO");
    let error = settings.expect_err("a FIFO is not a settings file");
    assert!(error.contains("not a regular file"), "{error}");
}

#[test]
fn a_loaded_panel_setting_hides_or_shows_hidden_files_in_both_panels() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    fs::write(temp.path().join(".hidden"), "dot file")?;
    let path = temp.path().join("config").join("settings.yaml");
    let mut app = app_in(&temp)?;
    let lists_hidden = |app: &App| {
        [&app.left_panel, &app.right_panel]
            .map(|panel| panel.entries.iter().any(|entry| entry.name == ".hidden"))
    };
    write_file(&path, "options: {Panel: off}\n")?;
    load(&mut app, &path)?;
    assert!(app.left_panel.options.hide_hidden && app.right_panel.options.hide_hidden);
    assert_eq!(lists_hidden(&app), [false, false]);
    write_file(&path, "options: {Panel: on}\n")?;
    load(&mut app, &path)?;
    assert_eq!(lists_hidden(&app), [true, true]);
    Ok(())
}

/// Save setup writes the pages changed in this session and keeps a page the
/// user changed in the file meanwhile; after a save, the file is the new
/// reference, so a later hand edit survives the next save too.
#[test]
fn save_setup_keeps_a_page_edited_by_hand_while_ffm_runs() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '8'}\n")?;
    let mut app = app_in(&temp)?;
    load(&mut app, &path)?;
    fs::write(&path, "options: {Editor: '2'}\n")?;
    app.settings.set(SettingsPage::Panel, "off").unwrap();
    save(&mut app)?;
    let expected = OptionsSection {
        options: PageValues(BTreeMap::from([
            (SettingsPage::Panel, "off".to_owned()),
            (SettingsPage::Editor, "2".to_owned()),
        ])),
    };
    let stored = document::read(&path)?.expect("saved");
    assert_eq!(stored.parse::<OptionsSection>()?, expected);
    fs::write(&path, "options: {Editor: '3', Panel: 'off'}\n")?;
    save(&mut app)?;
    assert_eq!(
        fs::read_to_string(&path)?,
        "options: {Editor: '3', Panel: 'off'}\n",
        "nothing changed since the last save"
    );
    Ok(())
}

#[test]
fn a_page_changed_by_hand_and_in_the_dialog_is_not_saved() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.yaml");
    fs::write(&path, "options: {Editor: '8'}\n")?;
    let mut app = app_in(&temp)?;
    load(&mut app, &path)?;
    fs::write(&path, "options: {Editor: '2'}\n")?;
    app.settings.set(SettingsPage::Editor, "6").unwrap();
    let error = save(&mut app).expect_err("both changed the Editor page");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml changed on disk: options.Editor was edited there and in \
         this session"
    );
    assert_eq!(fs::read_to_string(&path)?, "options: {Editor: '2'}\n");
    Ok(())
}

/// A legacy file in the way is named at startup and when Save setup refuses.
#[test]
fn a_legacy_settings_json_that_cannot_be_read_is_named() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("config").join("settings.yaml");
    fs::create_dir_all(temp.path().join("config").join("settings.json"))?;
    let mut app = app_in(&temp)?;
    restore(&mut app, path.clone());
    assert_eq!(
        app.status.to_string(),
        "Error: Settings: settings.json is not a regular file"
    );
    let error = save(&mut app).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.json is not a regular file"
    );
    assert!(!path.exists());
    Ok(())
}
