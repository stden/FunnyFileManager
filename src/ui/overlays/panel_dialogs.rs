//! Purpose: The panels' small dialogs: the Select/Deselect mask with its
//! offered text selected, the quit question, the delete warning that names
//! what it deletes, and the new-file name box.

use ratatui::{layout::Rect, style::Style, Frame};

use super::super::{dialog, palette, Cells};

pub(in crate::ui) fn render_mask_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &crate::models::MaskPrompt,
) {
    // Far's Select box: one field, and the mask it starts on is the
    // extension of whatever the cursor was standing on.
    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        prompt.purpose().title(),
        dialog::Ground::Prompt,
        &[
            dialog::Row::Label("Enter the &mask:"),
            dialog::Row::input(prompt.mask(), !prompt.on_cancel(), true),
        ],
        &[
            dialog::Button::new("&OK")
                .default_button()
                .focused(!prompt.on_cancel()),
            dialog::Button::new("&Cancel").focused(prompt.on_cancel()),
        ],
    ) {
        if prompt.is_offered() {
            select_field_text(f, prompt.mask(), x, y);
        }
        f.set_cursor_position((
            super::super::describe::caret_column(x, prompt.after_caret()),
            y,
        ));
    }
}

/// Draw `text`, which ends at the caret, as a selection: Far shows an offered
/// mask selected, and that is what tells the user typing will replace it.
fn select_field_text(f: &mut Frame<'_>, text: &str, end: u16, y: u16) {
    let width = u16::try_from(Cells::of(text).get()).unwrap_or(u16::MAX);
    let cells = Rect::new(end.saturating_sub(width), y, width, 1).intersection(f.area());
    let selected = Style::default()
        .bg(palette::FIELD_SELECTED_BG)
        .fg(palette::FIELD_SELECTED_FG);
    f.buffer_mut().set_style(cells, selected);
}

pub(super) fn render_quit_confirm(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    answer: crate::models::Confirm,
) {
    dialog::render_tracked(
        f,
        tracker,
        "Quit",
        dialog::Ground::Prompt,
        &[dialog::Row::Text("Do you want to quit FFM?")],
        &[
            dialog::Button::new("&Yes")
                .default_button()
                .focused(answer.is_yes()),
            dialog::Button::new("&No").focused(!answer.is_yes()),
        ],
    );
}

pub(super) fn render_delete_confirm(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    files: &[std::path::PathBuf],
) {
    // Far names the single file it is about and counts the rest: the
    // name is what the user checks before pressing Enter, and a bare
    // count is the one thing that cannot be checked.
    let mut message = vec![match files.len() {
        1 => "Do you wish to delete the file?".to_owned(),
        count => format!("Do you wish to delete {count} items?"),
    }];
    dialog::preview_lines(files, |path| super::super::entry_label(path), &mut message);

    dialog::render_warning(
        f,
        tracker,
        "Delete",
        &message,
        &[
            dialog::Button::new("Delete").default_button().focused(true),
            dialog::Button::new("Cancel"),
        ],
    );
}

pub(super) fn render_new_file_input(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    input: &str,
) {
    let input = super::super::tail_cells(input, Cells::columns(f.area().width.saturating_sub(9)));
    if let Some(cursor) = dialog::render_tracked(
        f,
        tracker,
        "Create New File (Shift+F4)",
        dialog::Ground::Prompt,
        &[
            dialog::Row::Label("Enter filename:"),
            dialog::Row::input(&input, true, false),
        ],
        &[
            dialog::Button::new("OK").default_button(),
            dialog::Button::new("Cancel"),
        ],
    ) {
        f.set_cursor_position(cursor);
    }
}
