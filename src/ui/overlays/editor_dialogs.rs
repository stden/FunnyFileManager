//! Purpose: What the editor opens over its text: the F7 search dialog and a
//! failed search, the encoding choice, the save question on exit, and the
//! warning before overwriting another program's write.

use ratatui::Frame;

use super::super::dialog;
use crate::models::ExitChoice;
use crate::search::SearchPrompt;

/// Far's F7 dialog: what to look for, how to match it, and which way to go.
fn render_search_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &SearchPrompt,
) -> Option<(u16, u16)> {
    use crate::search::{Case, SearchField, WordMatch};

    let rows = [
        dialog::Row::Label("&Search for"),
        dialog::Row::input(&prompt.query, prompt.field == SearchField::Query, false),
        dialog::Row::Divider,
        dialog::Row::checkbox(
            "&Case sensitive",
            prompt.case == Case::Match,
            prompt.field == SearchField::Case,
        ),
        dialog::Row::checkbox(
            "&Whole words",
            prompt.words == WordMatch::WholeWords,
            prompt.field == SearchField::Words,
        ),
    ];
    // Far gives the direction two buttons rather than a checkbox: the choice
    // and the command are then one keystroke instead of two.
    let buttons = search_buttons(prompt);
    dialog::render_tracked(
        f,
        tracker,
        "Search",
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    )
}

fn search_buttons(prompt: &SearchPrompt) -> [dialog::Button<'static>; 3] {
    use crate::search::SearchField;
    let buttons = [
        dialog::Button::new("Find pre&vious").focused(prompt.field == SearchField::FindPrevious),
        dialog::Button::new("Find &next")
            .default_button()
            .focused(prompt.field == SearchField::FindNext),
        dialog::Button::new("&Cancel").focused(prompt.field == SearchField::Cancel),
    ];
    buttons
}

pub(super) fn render_editor_dialog(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    editor: &crate::models::Editor,
    caret: (u16, u16),
) -> bool {
    use crate::models::EditorModal;
    let Some(modal) = editor.modal() else {
        return false;
    };
    match modal {
        EditorModal::Encoding(choice) => render_encoding_dialog(f, tracker, choice),
        EditorModal::Search(prompt) => {
            if let Some((x, y)) = render_search_prompt(f, tracker, prompt) {
                f.set_cursor_position((x, y));
            }
        }
        EditorModal::NotFound(needle) => render_search_miss(f, tracker, needle),
        EditorModal::ConfirmExit(choice) => render_exit_prompt(f, tracker, choice),
        EditorModal::ConfirmOverwrite(prompt) => {
            render_overwrite_prompt(f, tracker, prompt.answer);
        }
        EditorModal::Lsp => return super::super::editor_lsp::popup(f, editor, caret),
    }
    true
}

fn render_exit_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    choice: ExitChoice,
) {
    dialog::render_warning(
        f,
        tracker,
        "Editor",
        &["File has been modified. Save?".to_owned()],
        &[
            dialog::Button::new("Yes")
                .default_button()
                .focused(choice == ExitChoice::Save),
            dialog::Button::new("No").focused(choice == ExitChoice::Discard),
            dialog::Button::new("Cancel").focused(choice == ExitChoice::Cancel),
        ],
    );
}

/// A save found another program's write: say so before replacing it.
fn render_overwrite_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    answer: crate::models::Confirm,
) {
    dialog::render_warning(
        f,
        tracker,
        "Editor",
        &[
            "The file was changed by another program".to_owned(),
            "after it was opened. Overwrite it?".to_owned(),
        ],
        &[
            dialog::Button::new("Overwrite").focused(answer.is_yes()),
            dialog::Button::new("Cancel")
                .default_button()
                .focused(!answer.is_yes()),
        ],
    );
}

fn render_encoding_dialog(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    choice: crate::encoding::TextEncoding,
) {
    let rows: Vec<_> = crate::encoding::TextEncoding::ALL
        .iter()
        .map(|encoding| {
            dialog::Row::checkbox(encoding.label(), *encoding == choice, *encoding == choice)
        })
        .collect();
    dialog::render_tracked(
        f,
        tracker,
        "Encoding (arrows select)",
        dialog::Ground::Prompt,
        &rows,
        &[
            dialog::Button::new("&Use for saving").default_button(),
            dialog::Button::new("&Reload"),
            dialog::Button::new("&Cancel"),
        ],
    );
}

fn render_search_miss(f: &mut Frame<'_>, tracker: &mut crate::mouse::DialogMouse, needle: &str) {
    dialog::render_warning(
        f,
        tracker,
        "Search",
        &[format!("\"{needle}\" not found")],
        &[dialog::Button::new("Ok").default_button().focused(true)],
    );
}

#[cfg(test)]
mod tests_lsp;
