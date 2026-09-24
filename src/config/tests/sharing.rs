//! Unit tests: the typed settings and the Options pages share one file with
//! the user's own hand edits. What one reads is what it reads after the
//! other saves, and no save takes back a change it did not make.

use std::{fs, io, path::Path};

use tempfile::TempDir;

use crate::config::{patch, Enablement, LspSettings, Settings};
use crate::menu::SettingsPage;
use crate::models::App;
use crate::ops::menu::settings as options;

fn settings_path(temp: &TempDir) -> std::path::PathBuf {
    temp.path().join("ffm").join("settings.yaml")
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

fn app_in(temp: &TempDir) -> io::Result<App> {
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    Ok(App::from_panels(panel.clone(), panel))
}

/// The Options pages `pages` as a fresh session loads them from `path`.
fn load_pages(temp: &TempDir, path: &Path, pages: &[SettingsPage]) -> io::Result<Vec<String>> {
    let mut app = app_in(temp)?;
    options::load(&mut app, path)?;
    Ok(pages
        .iter()
        .map(|page| app.settings.get(*page).to_owned())
        .collect())
}

/// Read from the text, `Plugins: ~` was the command `~` until the exit
/// auto-save wrote it back as `null`, and then the menu offered "null".
#[test]
fn a_page_reads_the_same_before_and_after_the_typed_settings_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(
        &path,
        "options:\n  Masks:\n  Plugins: ~\n  Tree: +32\n  Descriptions: 0x10\n\
         system:\n  auto_save_setup: true\n",
    )?;
    let pages = [
        SettingsPage::Masks,
        SettingsPage::Plugins,
        SettingsPage::Tree,
        SettingsPage::Descriptions,
    ];
    let before = load_pages(&temp, &path, &pages)?;
    assert_eq!(before, ["", "", "32", "16"]);
    let mut typed = Settings::load_from(&path)?;
    typed.system.wipe_on_delete = true;
    typed.save_to(&path)?;
    let text = fs::read_to_string(&path)?;
    assert!(text.contains("\n  Plugins: null\n"), "{text}");
    assert_eq!(load_pages(&temp, &path, &pages)?, before);
    Ok(())
}

/// An empty typed text is a load error that names it, before Save setup
/// rewrites the file and after: never the text `null`.
#[test]
fn an_empty_typed_value_reads_the_same_before_and_after_save_setup() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(
        &path,
        "command_line:\n  prompt_format:\nviewer:\n  default_encoding: ~\noptions: {Editor: '8'}\n",
    )?;
    let before = Settings::load_from(&path).expect_err("an empty prompt is no text");
    assert!(
        before
            .to_string()
            .contains("command_line.prompt_format: invalid type: unit value"),
        "{before}"
    );
    let mut app = app_in(&temp)?;
    options::load(&mut app, &path)?;
    app.settings
        .set(SettingsPage::Editor, "2")
        .map_err(io::Error::other)?;
    options::save(&mut app)?;
    assert!(fs::read_to_string(&path)?.contains("  Editor: '2'\n"));
    let after = Settings::load_from(&path).expect_err("still no text");
    assert_eq!(after.to_string(), before.to_string());
    Ok(())
}

/// The exit auto-save used to put back what the user changed in the file
/// while FFM ran, in its own editor included.
#[test]
fn a_hand_edit_made_while_ffm_runs_survives_the_exit_auto_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut loaded = Settings::load_from(&path)?;
    write_file(&path, "system:\n  history_size: 9\n")?;
    loaded.save_to(&path)?;
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    loaded.system.wipe_on_delete = true;
    loaded.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 9);
    assert!(saved.system.wipe_on_delete);
    Ok(())
}

#[test]
fn a_setting_changed_on_disk_and_in_the_session_is_not_saved() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut loaded = Settings::load_from(&path)?;
    write_file(&path, "system:\n  history_size: 9\n")?;
    loaded.system.history_size = 7;
    let error = loaded.save_to(&path).expect_err("both changed it");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.yaml changed on disk: system.history_size was edited there \
         and in this session"
    );
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    Ok(())
}

/// A save compares with what was loaded, not with what the last save wrote:
/// a value this session saved and the user then edited by hand stops the
/// next save instead of being overwritten.
#[test]
fn a_hand_edit_of_a_value_this_session_saved_stops_the_next_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut settings = Settings::load_from(&path)?;
    settings.system.history_size = 7;
    settings.save_to(&path)?;
    assert_eq!(Settings::load_from(&path)?.system.history_size, 7);
    write_file(&path, "system:\n  history_size: 9\n")?;
    assert!(settings.save_to(&path).is_err());
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 9\n");
    Ok(())
}

/// A writer whose load failed holds the defaults. Once the user fixes the
/// file, its save writes what it changed and keeps what the user wrote.
#[test]
fn a_writer_whose_load_failed_keeps_what_the_user_fixed() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system: [\n")?;
    assert!(Settings::load_from(&path).is_err());
    let mut fallback = Settings::default();
    fallback.interface.clock = !fallback.interface.clock;
    write_file(
        &path,
        "system: {history_size: 9}\nlsp: {python: {enabled: false}}\n",
    )?;
    fallback.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 9);
    assert_eq!(saved.lsp.0["python"].enabled, Enablement::Disabled);
    assert_eq!(saved.interface.clock, fallback.interface.clock);
    Ok(())
}

#[test]
fn two_ffms_saving_keep_each_others_changes() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut first = Settings::load_from(&path)?;
    let mut second = Settings::load_from(&path)?;
    second.system.wipe_on_delete = true;
    second.save_to(&path)?;
    first.panel.show_hidden = false;
    first.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.system.history_size, 5);
    assert!(saved.system.wipe_on_delete);
    assert!(!saved.panel.show_hidden);
    Ok(())
}

/// `Editor` beside `editor` was a trap: the editor reads the page, and a
/// hand edit of `editor.tab_size` changed nothing. The pages have a key of
/// their own.
#[test]
fn no_page_sits_beside_a_typed_key_that_differs_only_in_case() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let mut typed = Settings::default();
    typed.editor.tab_size = 2;
    typed.save_to(&path)?;
    let mut app = app_in(&temp)?;
    options::load(&mut app, &path)?;
    app.settings
        .set(SettingsPage::Editor, "8")
        .map_err(io::Error::other)?;
    options::save(&mut app)?;
    let text = fs::read_to_string(&path)?;
    let top_level: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with([' ', '#', '-']))
        .collect();
    assert!(top_level.contains(&"editor:"), "{text}");
    assert!(top_level.contains(&"options:"), "{text}");
    assert!(text.contains("\noptions:\n  Editor: '8'\n"), "{text}");
    assert!(
        top_level.iter().all(|line| !line.starts_with("Editor")),
        "{text}"
    );
    Ok(())
}

/// What the Language servers dialog does on F2 for a session that loaded
/// `session`: write `change` of its servers into `path`, and adopt what the
/// file then holds.
fn dialog_save(
    session: &mut Settings,
    path: &Path,
    change: impl FnOnce(&mut LspSettings),
) -> io::Result<()> {
    let mut draft = session.lsp.clone();
    change(&mut draft);
    let edit =
        patch::Edit::new(&session.lsp, &draft).ok_or_else(|| io::Error::other("no change"))?;
    let saved = edit.save(path)?;
    session.adopt_saved_lsp(saved);
    Ok(())
}

fn python(servers: &mut LspSettings) -> &mut crate::config::ServerSettings {
    servers
        .0
        .get_mut("python")
        .expect("python is in the catalog")
}

/// After a dialog save the session holds the servers the file holds, so the
/// exit auto-save has nothing of `lsp` to write — and nothing else either.
#[test]
fn the_exit_save_writes_nothing_after_a_dialog_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).enabled = Enablement::Disabled;
    })?;
    let written = fs::read_to_string(&path)?;
    assert!(
        written.ends_with("system:\n  history_size: 5\nlsp:\n  python:\n    enabled: false\n"),
        "{written}"
    );
    session.save_to(&path)?;
    assert_eq!(fs::read_to_string(&path)?, written);
    Ok(())
}

/// A hand edit of the field the dialog just saved is the user's latest word:
/// the exit save neither refuses it as a conflict nor writes the session's
/// value over it.
#[test]
fn a_hand_edit_after_a_dialog_save_survives_the_exit_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).command = "dialog-pylsp".to_owned();
    })?;
    write_file(&path, "lsp:\n  python:\n    command: hand-pylsp\n")?;
    session.panel.show_hidden = !session.panel.show_hidden;
    session.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.lsp.0["python"].command, "hand-pylsp");
    assert_eq!(saved.panel.show_hidden, session.panel.show_hidden);
    Ok(())
}

/// The user undoes a dialog save by removing `lsp`: the exit save keeps the
/// file without it rather than pinning every catalog server into it.
#[test]
fn the_exit_save_never_pins_the_catalog_after_a_dialog_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).enabled = Enablement::Disabled;
    })?;
    write_file(&path, "system:\n  history_size: 5\n")?;
    session.panel.show_hidden = !session.panel.show_hidden;
    session.save_to(&path)?;
    let text = fs::read_to_string(&path)?;
    assert!(!text.contains("lsp"), "{text}");
    Ok(())
}
