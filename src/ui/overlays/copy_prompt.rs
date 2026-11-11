//! Purpose: Far's F5 Copy (and F6 Move) dialog: the destination, the access
//! rights, what to do about existing files, the transfer options and the
//! filter, in Far's groups.

use ratatui::Frame;

use super::super::dialog;

/// Far's F5 Copy dialog: where to, with what rights, and what to do about
/// files already there.
///
/// The groups are Far's, separated by rules: the destination, then the access
/// rights, then everything about collisions, then the filter, then the
/// buttons. The order is not cosmetic — it runs from the answer the user
/// always supplies to the ones they almost never change.
pub(super) fn render_copy_prompt(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &crate::copy::CopyPrompt,
) {
    use crate::copy::CopyField;

    let caption = prompt.caption();
    let on = |field: CopyField| prompt.field == field;

    let rows = copy_rows(prompt, &caption);

    let buttons = [
        dialog::Button::new(prompt.purpose.verb())
            .default_button()
            .focused(on(CopyField::Copy)),
        // Filter criteria do not have an editor yet. Keep Far's reserved place
        // visible but disabled rather than presenting a clickable dead button.
        dialog::Button::new("&Filter").enabled(false),
        dialog::Button::new("Ca&ncel").focused(on(CopyField::Cancel)),
    ];

    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        prompt.purpose.title(),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position((x, y));
    }
}

fn copy_rows<'a>(prompt: &'a crate::copy::CopyPrompt, caption: &'a str) -> Vec<dialog::Row<'a>> {
    use crate::copy::{AccessRights, CopyField};
    let on = |field| prompt.field == field;
    let mut rows = vec![
        dialog::Row::Label(caption),
        dialog::Row::line(&prompt.destination, on(CopyField::Destination), true),
        dialog::Row::Divider,
        dialog::Row::Radio {
            caption: "&Access rights:",
            options: &AccessRights::LABELS,
            selected: prompt.rights.index(),
            focused: on(CopyField::Rights),
        },
        dialog::Row::Divider,
        dialog::Row::Choice {
            caption: "Already e&xisting files:",
            value: prompt.existing.label(),
            focused: on(CopyField::Existing),
        },
    ];
    rows.extend(copy_option_rows(prompt));
    rows
}

fn copy_option_rows(prompt: &crate::copy::CopyPrompt) -> [dialog::Row<'_>; 5] {
    use crate::copy::CopyField;
    // A control the transfer cannot carry out is drawn greyed, exactly as the
    // ones a move does not offer are: it must not look tickable.
    let checkbox = |label: &'static str, checked: bool, field: CopyField| dialog::Row::Checkbox {
        label,
        checked,
        focused: prompt.field == field,
        enabled: prompt.allows(field),
    };
    [
        checkbox(
            "Preserve all &timestamps",
            prompt.timestamps,
            CopyField::Timestamps,
        ),
        checkbox(
            "Cop&y contents of symbolic links",
            prompt.symlink_contents,
            CopyField::SymlinkContents,
        ),
        checkbox(
            "Process &multiple destinations",
            prompt.multiple_destinations,
            CopyField::MultipleDestinations,
        ),
        dialog::Row::Divider,
        checkbox("&Use filter", prompt.filter, CopyField::Filter),
    ]
}
