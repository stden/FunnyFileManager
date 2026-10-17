//! Unit tests: the typed settings and the Options pages share one file with
//! the user's own hand edits. What one reads is what it reads after the
//! other saves, and no save takes back a change it did not make: hand edits
//! are in `sharing/hand_edits.rs`, the Language servers dialog's saves in
//! `sharing/dialog.rs`.

use std::{fs, io, path::Path};

use tempfile::TempDir;

use crate::config::Settings;
use crate::menu::SettingsPage;
use crate::models::App;
use crate::ops::menu::settings as options;

mod dialog;
mod hand_edits;

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
