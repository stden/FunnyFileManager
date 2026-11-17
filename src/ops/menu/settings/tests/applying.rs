//! Purpose: Settings that take effect at once: translations, descriptions, panel modes and hidden files.

use super::*;

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
