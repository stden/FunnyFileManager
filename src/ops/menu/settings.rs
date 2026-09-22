//! Read, apply, and atomically persist the main menu settings.

use crate::{
    menu::{MenuCommand, SettingsPage},
    models::{
        settings::{specification, Settings},
        App,
    },
};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub fn open(app: &mut App, page: SettingsPage) {
    let (title, _, caption) = specification(page);
    let value = app.settings.get(page).to_owned();
    super::prompt(app, MenuCommand::Settings(page), title, caption, &value);
}

pub fn load_default(app: &mut App) {
    load_from_environment(app, &std::env::var_os::<&str>);
}

fn load_from_environment(
    app: &mut App,
    variable: &dyn Fn(&'static str) -> Option<std::ffi::OsString>,
) {
    let base = variable("APPDATA")
        .or_else(|| variable("XDG_CONFIG_HOME"))
        .map(std::path::PathBuf::from)
        .or_else(|| variable("HOME").map(|home| std::path::PathBuf::from(home).join(".config")));
    if let Some(base) = base {
        if let Err(error) = load(app, &base.join("ffm/settings.json")) {
            app.set_status(crate::status::Status::Error(format!("Settings: {error}")));
        }
    }
}

pub fn submit(app: &mut App, page: SettingsPage, text: &str) -> io::Result<()> {
    let mut settings = app.settings.clone();
    settings.set(page, text).map_err(io::Error::other)?;
    install(app, settings, &[page])?;
    app.close_overlay();
    app.set_status(crate::status::Status::CommandOutput(
        "Setting applied. Use Save setup to keep it for the next session.".to_owned(),
    ));
    Ok(())
}

pub fn load(app: &mut App, path: &Path) -> io::Result<()> {
    let mut settings = Settings::with_file(path.to_owned());
    if path.exists() {
        if fs::metadata(path)?.len() > 1_048_576 {
            return Err(io::Error::other("Settings file exceeds 1 MiB"));
        }
        let values: std::collections::BTreeMap<SettingsPage, String> =
            serde_json::from_slice(&fs::read(path)?).map_err(io::Error::other)?;
        for (page, value) in values {
            settings.set(page, &value).map_err(io::Error::other)?;
        }
    }
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

pub fn save(app: &mut App) -> io::Result<()> {
    let path = app
        .settings
        .persistence
        .file()
        .ok_or_else(|| io::Error::other("No settings file configured for this session"))?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Settings path has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, app.settings.values())
        .map_err(io::Error::other)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(io::Error::other)?;
    app.set_status(crate::status::Status::CommandOutput(format!(
        "Setup saved to {}",
        path.display()
    )));
    Ok(())
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

fn translations(language: &str) -> io::Result<std::collections::BTreeMap<String, String>> {
    if language == "English" {
        return Ok(std::collections::BTreeMap::new());
    }
    if fs::metadata(language)?.len() > 1_048_576 {
        return Err(io::Error::other("Language dictionary exceeds 1 MiB"));
    }
    serde_json::from_slice(&fs::read(language)?).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
