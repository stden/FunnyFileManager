//! Read, apply, and atomically persist the main menu settings.
//!
//! The pages live in the settings file beside `config::Settings`; how the
//! two share it, and why a save may refuse, is in `config::document`.

use crate::{
    config::document,
    history::Persistence,
    menu::SettingsPage,
    models::{
        settings::{specification, OptionsSection, PageValues, Settings},
        App, MenuPrompt,
    },
    status::Status,
};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

pub fn open(app: &mut App, page: SettingsPage) {
    let (title, _, caption) = specification(page);
    let value = app.settings.get(page).to_owned();
    super::prompt(app, MenuPrompt::Settings(page), title, caption, &value);
}

/// Load the pages from the settings file under the config home, if the
/// environment names one.
pub fn load_default(app: &mut App) {
    if let Some(path) = crate::config::Settings::path() {
        restore(app, path);
    }
}

/// Load the pages from `path` at startup. A file that fails to load leaves
/// the current pages in place and says why on the status line. It still
/// becomes the save target, so Save setup reports why it will not overwrite
/// the file instead of claiming there is no file at all.
pub fn restore(app: &mut App, path: PathBuf) {
    if let Err(error) = load(app, &path) {
        app.settings.persistence = Persistence::File(path);
        app.set_status(Status::Error(format!("Settings: {error}")));
    }
}

pub fn submit(app: &mut App, page: SettingsPage, text: &str) -> io::Result<()> {
    let mut settings = app.settings.clone();
    settings.set(page, text).map_err(io::Error::other)?;
    install(app, settings, &[page])?;
    app.close_overlay();
    app.set_status(Status::CommandOutput(
        "Setting applied. Use Save setup to keep it for the next session.".to_owned(),
    ));
    Ok(())
}

pub fn load(app: &mut App, path: &Path) -> io::Result<()> {
    let mut settings = Settings::with_file(path.to_owned());
    if let Some(stored) = document::read(path)? {
        let OptionsSection {
            options: PageValues(values),
        } = stored.parse()?;
        for (page, value) in values {
            settings
                .set(page, &value)
                .map_err(|error| io::Error::other(format!("{}: {error}", page.title())))?;
        }
    }
    settings.mark_saved();
    install(
        app,
        settings,
        &[
            SettingsPage::Panel,
            SettingsPage::Descriptions,
            SettingsPage::Language,
            SettingsPage::PanelModes,
        ],
    )
}

fn install(app: &mut App, settings: Settings, pages: &[SettingsPage]) -> io::Result<()> {
    let descriptions_changed = [SettingsPage::Information, SettingsPage::FolderDescriptions]
        .into_iter()
        .any(|page| app.settings.get(page) != settings.get(page));
    let mut staged = App::from_panels(app.left_panel.clone(), app.right_panel.clone());
    staged.settings = settings;
    for page in pages {
        apply(&mut staged, *page)?;
    }
    app.settings = staged.settings;
    app.left_panel = staged.left_panel;
    app.right_panel = staged.right_panel;
    if descriptions_changed {
        crate::ops::quickview::reload(app);
    }
    Ok(())
}

/// Save setup: write the pages changed since the last load or save into the
/// settings file, keeping every other key and every page edited by hand.
/// Refused, with the file untouched, when it does not parse or changed a page
/// this session changed too.
pub fn save(app: &mut App) -> io::Result<()> {
    let path = app
        .settings
        .persistence
        .file()
        .ok_or_else(|| io::Error::other("No settings file configured for this session"))?
        .to_owned();
    document::update(
        &path,
        &section(app.settings.saved_values()),
        &section(app.settings.values()),
    )?;
    app.settings.mark_saved();
    app.set_status(Status::SetupSaved(path));
    Ok(())
}

/// `values` as the settings file nests them.
fn section(values: &BTreeMap<SettingsPage, String>) -> OptionsSection {
    OptionsSection {
        options: PageValues(values.clone()),
    }
}

fn apply(app: &mut App, page: SettingsPage) -> io::Result<()> {
    match page {
        SettingsPage::Language => {
            app.settings.translations = translations(app.settings.get(page))?;
        }
        SettingsPage::Descriptions => {
            app.left_panel.options.description_file = app.settings.get(page).to_owned();
            app.right_panel.options.description_file = app.settings.get(page).to_owned();
            crate::utils::refresh_panels(app)?;
        }
        SettingsPage::Panel => {
            let hide = !app.settings.enabled(page);
            app.left_panel.options.hide_hidden = hide;
            app.right_panel.options.hide_hidden = hide;
            crate::utils::refresh_panels(app)?;
        }
        SettingsPage::PanelModes => {
            let selected = app.settings.get(page);
            let mode = crate::models::PanelView::ALL
                .into_iter()
                .find(|mode| mode.label() == selected)
                .ok_or_else(|| io::Error::other("Unknown panel mode"))?;
            crate::ops::panel_details::set_view(&mut app.left_panel, mode)?;
            crate::ops::panel_details::set_view(&mut app.right_panel, mode)?;
        }
        _ => {}
    }
    Ok(())
}

fn translations(language: &str) -> io::Result<BTreeMap<String, String>> {
    if language == "English" {
        return Ok(BTreeMap::new());
    }
    read_json(Path::new(language), "Language dictionary")
}

/// Read a language dictionary: a JSON map from label to translation.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path, what: &str) -> io::Result<T> {
    let text = crate::fileload::read_small_file(path, what)?;
    serde_json::from_str(&text).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
