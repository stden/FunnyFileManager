//! History, filtering, screen selection and host information commands.

use crate::models::{App, ChoiceTarget, MenuChoice, MenuPrompt, ViewMode};
use std::io;

pub(super) fn open_filter(app: &mut App) {
    let filter = app.get_active_panel_ref().options.filter.clone();
    super::prompt(
        app,
        MenuPrompt::Filter,
        "File panel filter",
        "Filename mask; empty shows all files. Directories remain visible.",
        &filter,
    );
}

pub(super) fn filter(app: &mut App, text: &str) -> io::Result<()> {
    let panel = app.get_active_panel();
    let previous = std::mem::replace(&mut panel.options.filter, text.to_owned());
    if let Err(error) = panel.load_entries() {
        panel.options.filter = previous;
        return Err(error);
    }
    app.close_overlay();
    Ok(())
}

pub(super) fn folder_history(app: &mut App) {
    app.record_folder_visits();
    let rows = app
        .folder_history
        .newest_first()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::Directory(path.to_path_buf()),
        })
        .collect();
    super::choices(app, "Folders history", rows);
}

/// Far's Alt+F11: the files viewed (F3) and edited (F4, Alt+F4), newest
/// first — the persisted history every one of those keys records, so the
/// list also reaches back into earlier sessions.
pub(super) fn view_history(app: &mut App) {
    let rows = app
        .view_history
        .newest_first()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::File(path.to_path_buf()),
        })
        .collect();
    super::choices(app, "File view history", rows);
}

pub(super) fn screens(app: &mut App) {
    super::choices(
        app,
        "Screens",
        vec![
            MenuChoice {
                label: "File panels".to_owned(),
                target: ChoiceTarget::Screen(ViewMode::Panels),
            },
            MenuChoice {
                label: "Console transcript".to_owned(),
                target: ChoiceTarget::Screen(ViewMode::Console),
            },
        ],
    );
}

pub(super) fn devices(app: &mut App) {
    let rows = App::get_drives()
        .into_iter()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::Directory(path),
        })
        .collect();
    super::choices(app, "Mounted devices", rows);
}
