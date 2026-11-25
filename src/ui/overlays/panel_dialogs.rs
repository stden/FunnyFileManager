//! Purpose: The panels' small dialogs: the Select/Deselect mask with its
//! offered text selected, the quit question, the delete warning that names
//! what it deletes, and the new-file name box.

use ratatui::Frame;

use super::super::{dialog, Cells};

/// What F8's or Alt+Del's question calls the one entry it is about — Far's
/// "delete the folder?" — by its kind as the active panel listed it. A path
/// the panel does not list, or any other dialog, says "file".
pub(super) fn single_target_noun(app: &crate::models::App) -> &'static str {
    use crate::models::{EntryKind, Overlay};

    let (Overlay::DeleteConfirm(files) | Overlay::WipeConfirm(files)) = &app.overlay else {
        return "file";
    };
    let kind = match files.as_slice() {
        [only] => app.get_active_panel_ref().kind_of(only),
        _ => None,
    };
    match kind {
        Some(EntryKind::Directory) => "folder",
        Some(
            EntryKind::SymlinkToFile | EntryKind::SymlinkToDirectory | EntryKind::BrokenSymlink,
        ) => "link",
        Some(EntryKind::File) | None => "file",
    }
}

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
            dialog::Row::line(prompt.line(), !prompt.on_cancel(), true),
        ],
        &[
            dialog::Button::new("&OK")
                .default_button()
                .focused(!prompt.on_cancel()),
            dialog::Button::new("&Cancel").focused(prompt.on_cancel()),
        ],
    ) {
        f.set_cursor_position((x, y));
    }
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
    noun: &str,
) {
    // Far names the single entry it is about — a file, a folder or a link —
    // and counts the rest: the name is what the user checks before pressing
    // Enter, and a bare count is the one thing that cannot be checked.
    let mut message = vec![match files.len() {
        1 => format!("Do you wish to delete the {noun}?"),
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
    input: &crate::lineedit::LineEditor,
) {
    let room = Cells::columns(f.area().width.saturating_sub(9));
    // A name wider than the screen shows its end, caret there; one that fits
    // shows its caret and selection where they are.
    let clipped = super::super::tail_cells(input.text(), room);
    let field = if Cells::of(input.text()).get() <= room.get() {
        dialog::Row::line(input, true, false)
    } else {
        dialog::Row::input(&clipped, true, false)
    };
    if let Some(cursor) = dialog::render_tracked(
        f,
        tracker,
        "Create New File (Shift+F4)",
        dialog::Ground::Prompt,
        &[dialog::Row::Label("Enter filename:"), field],
        &[
            dialog::Button::new("OK").default_button(),
            dialog::Button::new("Cancel"),
        ],
    ) {
        f.set_cursor_position(cursor);
    }
}
