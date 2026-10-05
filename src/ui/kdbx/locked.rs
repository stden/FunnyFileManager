//! Purpose: The `.kdbx` unlock prompt drawn while the vault is locked: the
//! master password field (bullets unless `◉` is toggled), the key file,
//! the last error, and the field's text clipped to the dialog's width.

use ratatui::Frame;

use super::super::dialog;

pub(super) fn render_locked(
    f: &mut Frame<'_>,
    name: &str,
    prompt: &crate::kdbx::UnlockPrompt,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let password = clipped_password(f.area(), name, prompt, tracker.offset);
    let detail = locked_details(prompt);
    let mut rows = vec![
        dialog::Row::Label("Master password:"),
        dialog::Row::input(&password, true, false),
    ];
    rows.extend(detail.iter().map(|text| dialog::Row::Text(text)));
    let buttons = [
        dialog::Button::new("Unlock").default_button(),
        dialog::Button::new("◉"),
        dialog::Button::new("Cancel"),
    ];
    if let Some(position) = dialog::render_tracked(
        f,
        tracker,
        &format!("KeePass: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position(position);
    }
}

fn clipped_password(
    screen: ratatui::layout::Rect,
    name: &str,
    prompt: &crate::kdbx::UnlockPrompt,
    offset: (i32, i32),
) -> String {
    let rows = [
        dialog::Row::Label("Master password:"),
        dialog::Row::input("", true, false),
    ];
    let buttons = [
        dialog::Button::new("Unlock").default_button(),
        dialog::Button::new("◉"),
        dialog::Button::new("Cancel"),
    ];
    let width = dialog::place(screen, &format!("KeePass: {name}"), &rows, &buttons, offset)
        .map_or(0, |placement| {
            usize::from(placement.body.width.saturating_sub(1))
        });
    prompt.visible_text().chars().take(width).collect()
}

fn locked_details(prompt: &crate::kdbx::UnlockPrompt) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(keyfile) = prompt.keyfile() {
        details.push(format!("Key file: {}", keyfile.display()));
    }
    if let Some(error) = prompt.error() {
        details.push(error.to_owned());
    }
    details.push("Enter Unlock  ·  Esc Cancel".to_owned());
    details
}
