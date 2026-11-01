//! Purpose: Far's Make folder dialog (F7) and its Rename box (Shift+F6): the
//! name field, and for F7 the link type, its target and multiple names.

use ratatui::Frame;

use super::super::dialog;

/// Far's Make folder dialog (F7), and its Rename box (Shift+F6).
///
/// The same dialog, because the name field, the buttons and the cancelling are
/// the same. F7 adds the group Far puts under the rule: what kind of link to
/// make, what it points at, and whether the name field is a list. Rename has
/// none of that, so the rows are simply not drawn — unlike a greyed control,
/// which says "not for this answer", an absent one says "not for this key".
pub(super) fn render_name_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &crate::models::NamePrompt,
) {
    use crate::models::{NameField, NamePurpose};

    let on = |field: NameField| prompt.field() == field;
    let question = match prompt.purpose() {
        NamePurpose::Rename => format!(
            "Rename {} to:",
            crate::ui::printable_name(prompt.original())
        ),
        NamePurpose::MakeFolder => "Create the &folder:".to_owned(),
        NamePurpose::Link => "Create the &link:".to_owned(),
    };
    let rows = name_prompt_rows(prompt, &question);

    let buttons = [
        dialog::Button::new("&OK")
            .default_button()
            .focused(on(NameField::Ok)),
        dialog::Button::new("&Cancel").focused(on(NameField::Cancel)),
    ];

    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        prompt.purpose().title(),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position((x, y));
    }
}

/// The question, the name, and whichever controls this purpose offers.
fn name_prompt_rows<'a>(
    prompt: &'a crate::models::NamePrompt,
    question: &'a str,
) -> Vec<dialog::Row<'a>> {
    use crate::models::NameField;
    let on = |field: NameField| prompt.field() == field;
    let mut rows = vec![
        dialog::Row::Label(question),
        dialog::Row::line(prompt.name_line(), on(NameField::Name), true),
    ];
    if prompt.allows(NameField::Link) {
        rows.extend(link_option_rows(prompt));
    }
    if prompt.allows(NameField::Multiple) {
        rows.push(dialog::Row::checkbox(
            "Process &multiple names",
            prompt.multiple(),
            on(NameField::Multiple),
        ));
    }
    rows
}

/// The link controls F7 and Alt+F6 share: what kind, and what it points at.
fn link_option_rows(prompt: &crate::models::NamePrompt) -> [dialog::Row<'_>; 3] {
    use crate::models::NameField;
    let on = |field| prompt.field() == field;
    [
        dialog::Row::Divider,
        dialog::Row::Choice {
            caption: "&Link type:",
            value: prompt.link().label(),
            focused: on(NameField::Link),
        },
        dialog::Row::field(
            "&Target:",
            prompt.target_line(),
            on(NameField::Target),
            prompt.allows(NameField::Target),
        ),
    ]
}
