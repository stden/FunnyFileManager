use super::super::dialog;
use crate::kdbx::{EditField, EntryForm, FormMode};
use ratatui::Frame;

pub(super) fn render(f: &mut Frame<'_>, path: &str, form: &EntryForm) {
    let password = form.password_visible().to_owned();
    let mut rows = fields(form, &password);
    append_help(&mut rows, form);
    if let Some(position) = dialog::render_at(
        f,
        &format!("Edit KeePass entry: {} — {}", path, form.origin_title()),
        dialog::Ground::Prompt,
        &rows,
        &[],
        (0, 0),
    ) {
        f.set_cursor_position(position);
    }
}

fn fields<'a>(form: &'a EntryForm, password: &'a str) -> Vec<dialog::Row<'a>> {
    let fields = [
        (EditField::Title, "Title", form.title()),
        (EditField::Username, "User", form.username()),
        (EditField::Url, "URL", form.url()),
        (EditField::Notes, "Notes", form.notes()),
        (EditField::Password, "Password", password),
    ];
    let mut rows = Vec::new();
    for (field, label, value) in fields {
        rows.push(dialog::Row::Label(label));
        rows.push(dialog::Row::input(value, form.focus() == field, false));
    }
    rows
}

fn append_help<'a>(rows: &mut Vec<dialog::Row<'a>>, form: &'a EntryForm) {
    rows.push(dialog::Row::Label(
        "Tab Fields | F2 Save | Ctrl+C Copy password | Ctrl+Z Undo | Esc Back",
    ));
    rows.push(dialog::Row::Label(
        "An encrypted .bak copy is saved before writing.",
    ));
    if let Some(error) = form.error() {
        rows.push(dialog::Row::Text(error));
    }
    if form.mode() == FormMode::ConfirmSave {
        rows.push(dialog::Row::Label(
            "Save changes? Enter/Y Save | N Discard | Esc Continue editing",
        ));
    }
}
