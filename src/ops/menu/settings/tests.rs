use super::*;

#[test]
fn default_configuration_respects_environment_precedence_and_reports_invalid_files(
) -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    for variable in ["APPDATA", "XDG_CONFIG_HOME", "HOME"] {
        load_from_environment(&mut app, &|key| {
            (key == variable).then(|| temp.path().as_os_str().to_owned())
        });
        let relative = if variable == "HOME" {
            ".config/ffm/settings.json"
        } else {
            "ffm/settings.json"
        };
        assert_eq!(
            app.settings.persistence.file(),
            Some(temp.path().join(relative).as_path())
        );
    }
    fs::create_dir_all(temp.path().join("ffm"))?;
    fs::write(temp.path().join("ffm/settings.json"), "broken")?;
    load_from_environment(&mut app, &|_| Some(temp.path().as_os_str().to_owned()));
    assert!(matches!(app.status, crate::status::Status::Error(_)));
    let previous = app.settings.persistence.clone();
    load_from_environment(&mut app, &|_| None);
    assert_eq!(app.settings.persistence, previous);
    Ok(())
}

#[test]
fn unavailable_language_rolls_back_loaded_panel_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    fs::write(temp.path().join(".hidden"), "keep visible")?;
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    let path = temp.path().join("settings.json");
    let values = std::collections::BTreeMap::from([
        (SettingsPage::Panel, "off".to_owned()),
        (
            SettingsPage::Language,
            temp.path().join("missing.json").display().to_string(),
        ),
    ]);
    fs::write(&path, serde_json::to_vec(&values)?)?;
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
    let path = temp.path().join("config/settings.json");
    let mut app = App::new()?;
    load(&mut app, &path)?;
    app.settings.set(SettingsPage::Editor, "8").unwrap();
    save(&mut app)?;
    let mut restored = App::new()?;
    load(&mut restored, &path)?;
    assert_eq!(restored.settings.number(SettingsPage::Editor), 8);
    assert!(restored.settings.enabled(SettingsPage::Panel));
    Ok(())
}

#[test]
fn invalid_setup_does_not_replace_the_current_settings() -> io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp.path().join("settings.json");
    fs::write(&path, r#"{"Editor":"0"}"#)?;
    let mut app = App::new()?;
    assert!(load(&mut app, &path).is_err());
    assert_eq!(app.settings.number(SettingsPage::Editor), 4);
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
fn load_default_reads_whatever_the_process_environment_names() {
    let temp = tempfile::TempDir::new().expect("dir");
    let panel = crate::models::Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    load_default(&mut app);
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
    app.settings.persistence = crate::history::Persistence::SessionOnly;
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
            let panel =
                crate::models::Panel::from_entries(std::path::PathBuf::from("/"), Vec::new());
            let mut app = App::with_panels(panel.clone(), panel);
            load(&mut app, &fifo).map_err(|error| error.to_string())
        })
        .expect("loading settings must not block on a FIFO");
    let error = settings.expect_err("a FIFO is not a settings file");
    assert!(error.contains("not a regular file"), "{error}");
}
