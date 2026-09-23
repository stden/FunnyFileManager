//! Modal overlay rendering: help, viewer, built-in editor, dialogs.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::code::editor_window;
use super::dialog;
use super::palette;
use super::{centered_rect, far_block, head_cells, screen_u16, Cells, EditorLine, FAR_BLUE};
use crate::models::{App, ExitChoice, Overlay};
use crate::search::SearchPrompt;

#[cfg(test)]
mod tests;

/// Far's F7 dialog: what to look for, how to match it, and which way to go.
fn render_search_prompt(
    f: &mut Frame,
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

/// What one line of the information panel is.
enum Info {
    Summary(String),
    Heading(&'static str),
    Detail(String),
}

/// Far's Ctrl+L panel, answering "where is the space and what is all this".
///
/// Three sections because they answer three different questions: the totals say
/// how much there is, the folders say *where* it is, and the categories say
/// *what* it is. One flat list would answer none of them.
fn render_information(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    stats: &crate::stats::DirectoryStats,
) {
    let lines = information_lines(stats);
    // A heading opens a new group, so it is preceded by the rule that divides
    // one from the last — except the first, which follows the summary's own.
    let mut rows = Vec::new();
    for line in &lines {
        match line {
            Info::Summary(text) => rows.push(dialog::Row::Text(text)),
            Info::Heading(title) => {
                rows.push(dialog::Row::Divider);
                rows.push(dialog::Row::Label(title));
            }
            Info::Detail(text) => rows.push(dialog::Row::Label(text)),
        }
    }

    dialog::render_tracked(
        f,
        tracker,
        "Information",
        dialog::Ground::Prompt,
        &rows,
        &[dialog::Button::new("&Ok").default_button().focused(true)],
    );
}

fn information_lines(stats: &crate::stats::DirectoryStats) -> Vec<Info> {
    use super::panel::abbreviate_bytes;
    use crate::stats::Coverage;

    // No totals at all for a folder that could not be read: its zeros would
    // read as an empty folder.
    if let Coverage::Unreadable(reason) = &stats.coverage {
        return vec![Info::Summary(format!("Cannot read this folder: {reason}"))];
    }
    let mut lines = vec![Info::Summary(format!(
        "{} in {} files, {} folders",
        abbreviate_bytes(stats.total.bytes()),
        stats.files,
        stats.directories
    ))];
    match stats.coverage {
        Coverage::Partial => lines.push(Info::Summary(
            "measured to the walk limit — totals are lower bounds".to_owned(),
        )),
        Coverage::Unread(folders) => lines.push(Info::Summary(format!(
            "{folders} folder(s) could not be read — totals are lower bounds"
        ))),
        Coverage::Complete | Coverage::Unreadable(_) => {}
    }

    append_information_sections(&mut lines, stats);

    lines
}

fn append_information_sections(lines: &mut Vec<Info>, stats: &crate::stats::DirectoryStats) {
    if !stats.children.is_empty() {
        lines.push(Info::Heading("Largest folders"));
        lines.extend(stats.children.iter().map(|child| {
            Info::Detail(information_entry(
                &child.name,
                child.size.bytes(),
                Some(child.files),
            ))
        }));
    }
    if !stats.by_kind.is_empty() {
        lines.push(Info::Heading("By content"));
        lines.extend(stats.by_kind.iter().map(|kind| {
            Info::Detail(information_entry(
                kind.kind.label(),
                kind.size.bytes(),
                Some(kind.files),
            ))
        }));
    }
    if !stats.largest.is_empty() {
        lines.push(Info::Heading("Largest files"));
        lines.extend(
            stats
                .largest
                .iter()
                .map(|file| Info::Detail(information_entry(&file.path, file.size.bytes(), None))),
        );
    }
}

/// Columns given to a name before its size.
fn information_entry(name: &str, size: u64, count: Option<usize>) -> String {
    use super::panel::abbreviate_bytes;
    const NAME: usize = 30;
    let name = head_cells(name, Cells::new(NAME));
    let padding = " ".repeat(NAME.saturating_sub(Cells::of(&name).get()));
    match count {
        Some(files) => format!("{name}{padding} {:>9}  {files}", abbreviate_bytes(size)),
        None => format!("{name}{padding} {:>9}", abbreviate_bytes(size)),
    }
}

/// File names the delete dialog lists before it starts counting instead.
const DELETE_PREVIEW: usize = 5;

/// Rows the editor spends on chrome: its top bar, and nothing else.
const EDITOR_BAR_HEIGHT: u16 = 1;

/// Far's editor bar: the file on the left, where the cursor is on the right.
///
/// `Ch` is the code point under the cursor, which is the reason the bar is
/// worth its row — it is the one fact about a file you cannot get by looking
/// at it, and the one you need when a file will not parse.
fn render_editor_bar(f: &mut Frame, editor: &crate::models::Editor, area: Rect, line: &str) {
    let text = editor_bar_text(editor, area.width, line);
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(text))).style(
            Style::default()
                .bg(palette::EDITOR_BAR_BG)
                .fg(palette::EDITOR_BAR_FG)
                .add_modifier(Modifier::BOLD),
        ),
        Rect {
            height: EDITOR_BAR_HEIGHT,
            ..area
        },
    );
}

fn editor_bar_text(editor: &crate::models::Editor, width: u16, line: &str) -> String {
    let marker = if editor.state.is_modified() { " *" } else { "" };
    let column = editor.cursor_column().chars();
    let code = line.chars().nth(column).map_or(0, u32::from);
    let position = format!(
        " Ln {}/{}  Col {}  Ch {code:#06x} ",
        editor.cursor_line + 1,
        editor.lines.len(),
        column + 1,
    );

    let width = Cells::columns(width);
    let label = editor.notice.as_ref().map_or_else(
        || super::editor_lsp::label(editor, marker),
        |notice| format!(" {notice}"),
    );
    // An operation's result must remain visible on a narrow terminal. Keep
    // the cursor details alongside it only when both fit in the header.
    let position = if editor.notice.is_some() && Cells::of(&label) + Cells::of(&position) > width {
        String::new()
    } else {
        position
    };
    let name = head_cells(&label, width.saturating_sub(Cells::of(&position)));
    let gap = width
        .saturating_sub(Cells::of(&name) + Cells::of(&position))
        .get();

    format!("{name}{}{position}", " ".repeat(gap))
}

pub fn render_overlays(f: &mut Frame, app: &mut App) {
    if !app.overlay.is_active() {
        // No dialog this frame, so last frame's rectangles describe nothing.
        // Leaving them would send a click on the panels into a closed window.
        app.dialog_mouse.clear();
    }
    match &mut app.overlay {
        // Nothing to draw on top. The command line is part of the main
        // layout, and the menu hangs from the top row rather than floating in
        // the middle of the screen — both are drawn by whoever owns that row.
        Overlay::None | Overlay::CommandLine(_) | Overlay::Menu(_) => {}
        Overlay::Wsl(dialog) => super::wsl::render(f, dialog),
        Overlay::PathEditor(dialog) => super::path_env::render(f, dialog),
        Overlay::MenuDialog(dialog) => super::menu_dialog::render(f, dialog, &mut app.dialog_mouse),
        Overlay::Help => render_help(f),
        Overlay::Viewer(viewer) => super::viewer::render_viewer(f, viewer),
        Overlay::Editor(editor) => render_editor(f, editor, &mut app.dialog_mouse),
        Overlay::DriveSelection {
            side: target_panel,
            selected_index,
            rows,
        } => super::drives::render(f, *target_panel, rows, *selected_index),
        Overlay::NamePrompt(prompt) => render_name_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::MaskPrompt(prompt) => render_mask_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::Information(stats) => render_information(f, &mut app.dialog_mouse, stats),
        Overlay::CopyPrompt(prompt) => render_copy_prompt(f, &mut app.dialog_mouse, prompt),
        Overlay::QuitConfirm(answer) => render_quit_confirm(f, &mut app.dialog_mouse, *answer),
        Overlay::DeleteConfirm(files) => render_delete_confirm(f, &mut app.dialog_mouse, files),
        Overlay::NewFileInput(input) => render_new_file_input(f, &mut app.dialog_mouse, input),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(overlay) => super::kdbx::render_kdbx(f, &overlay.0, &mut app.dialog_mouse),
        other => render_menu_dialog(f, other, &mut app.dialog_mouse),
    }
}

/// The Files-menu dialogs the completed F9 menu brought, one owner each.
///
/// A second and third function only to keep each under the line limit; the
/// split follows the menu headings.
fn render_menu_dialog(
    f: &mut Frame,
    overlay: &mut Overlay,
    tracker: &mut crate::mouse::DialogMouse,
) {
    match overlay {
        Overlay::WipeConfirm(dialog) => super::wipe::render(f, tracker, dialog),
        Overlay::ArchivePrompt(dialog) => super::archive_prompt::render(f, tracker, dialog),
        Overlay::ExtractPrompt(dialog) => super::extract_prompt::render(f, tracker, dialog),
        Overlay::ArchiveCommands(dialog) => super::archive_commands::render(f, tracker, dialog),
        Overlay::Attributes(dialog) => super::attributes::render(f, tracker, dialog),
        Overlay::ApplyCommand(dialog) => super::apply::render(f, tracker, dialog),
        Overlay::Describe(dialog) => super::describe::render(f, tracker, dialog),
        other => render_commands_dialog(f, other, tracker),
    }
}

/// The Commands- and Options-menu dialogs. Every variant is drawn by one of
/// the three routers; the fallback here is only for the arms above.
fn render_commands_dialog(
    f: &mut Frame,
    overlay: &mut Overlay,
    tracker: &mut crate::mouse::DialogMouse,
) {
    match overlay {
        Overlay::FindFile(dialog) => super::findfile::render(f, tracker, dialog),
        Overlay::CommandHistory(dialog) => super::history_list::render(f, tracker, dialog),
        Overlay::FindFolder(dialog) => super::findfolder::render(f, tracker, dialog),
        Overlay::ViewHistory(dialog) => super::view_history::render(f, tracker, dialog),
        Overlay::FolderHistory(dialog) => super::folder_history::render(f, tracker, dialog),
        Overlay::UserMenu(dialog) => super::usermenu::render(f, tracker, dialog),
        Overlay::Associations(dialog) => super::associations::render(f, tracker, dialog),
        Overlay::FolderShortcuts(dialog) => super::shortcuts::render(f, tracker, dialog),
        Overlay::Filter(dialog) => super::filter::render(f, tracker, dialog),
        Overlay::Screens(dialog) => super::screens::render(f, tracker, dialog),
        Overlay::Tasks(dialog) => super::tasks::render(f, tracker, dialog),
        Overlay::Hotplug(dialog) => super::hotplug::render(f, tracker, dialog),
        Overlay::SortModes(dialog) => super::sortmenu::render(f, tracker, dialog),
        Overlay::Settings(dialog) => super::settings::render(f, tracker, dialog),
        Overlay::DiskCleanup(dialog) => super::disk_cleanup::render(f, tracker, dialog),
        _ => {}
    }
}

const HELP_LINES: &[&str] = &[
    "Far Manager style navigation:",
    "F1: Toggle this help window",
    "F2: Open the user command menu",
    "F3: View selected file (read-only)",
    "F4: Edit selected file (built-in editor, F2 saves)",
    "  in the editor: F7 search, Shift+F7 search again",
    "Alt+F4: Edit selected file with $EDITOR or VS Code",
    "F5: Copy selected entry to other panel",
    "F6: Move/Rename selected entry to other panel",
    "F7: Create a new directory in active panel",
    "F8: Delete selected entry",
    "F9: Open the menu bar",
    "F10: Quit (or type `quit`)",
    "F11: MCP Server status (AI tools)",
    "F12: Switch panels / console screen",
    "Ctrl+O: Show the console screen (press again for the panels)",
    "Tab: switch active panel",
    "PgUp/PgDn: move the selection one screen up/down",
    ":  - command line (Tab completes verbs, args & cd paths)",
    ":mcp - MCP server status & installation",
    ":winmsg - recent Windows system warnings/errors",
    ":winperf - Windows performance summary",
];

fn render_help(f: &mut Frame) {
    let area = centered_rect(80, 80, f.area());
    dialog::render_shadow(f, area, f.area());

    let content = HELP_LINES
        .iter()
        .copied()
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<Line>>();

    let style = Style::default().fg(palette::HELP_FG).bg(palette::HELP_BG);
    let block = far_block()
        .title(Line::from(" Help ").centered())
        .border_style(style)
        .style(style);

    let paragraph = Paragraph::new(content).block(block).style(style);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_editor(
    f: &mut Frame,
    editor: &crate::models::Editor,
    tracker: &mut crate::mouse::DialogMouse,
) {
    // Full screen, as Far's editor is. A window inset inside the panels
    // leaves the columns showing round the edges, which reads as a
    // dialog floating over the file manager rather than as the editor
    // you switched into — and it costs the file ten columns of width.
    let area = f.area();
    // One row of chrome, not a frame. Far's editor spends its top row
    // on the file name and the cursor position and gives every other
    // cell to the file — a box would cost two rows and two columns to
    // draw a line nobody reads.
    let inner_h = usize::from(area.height.saturating_sub(EDITOR_BAR_HEIGHT));
    let inner_w = Cells::columns(area.width);

    // Stateless scrolling: keep the cursor roughly centered vertically
    // and always visible horizontally — no scroll state to desync.
    let scroll_y = editor.cursor_line.saturating_sub(inner_h / 2);

    // `cursor_col` is a CHAR index into the buffer, which is what
    // editing needs — but the screen budgets in CELLS, and a CJK glyph
    // spends two. Scrolling and positioning by the char index put both
    // the window and the cursor in the wrong column on any non-ASCII
    // line; convert once, here, and work in cells below.
    let cursor_line_text = editor
        .lines
        .get(editor.cursor_line)
        .map_or("", String::as_str);
    let cursor_line = EditorLine {
        text: cursor_line_text,
        tab: editor.tab_size,
    };
    let cursor_cell = cursor_line.cells_before(editor.cursor_column());
    let scroll_x = cursor_cell.saturating_sub(inner_w.saturating_sub(Cells::new(1)));

    render_editor_bar(f, editor, area, cursor_line_text);

    render_editor_body(f, editor, area, scroll_y, scroll_x);

    // Position against the cells actually dropped, not against `scroll_x`:
    // a double-width glyph straddling the cut is dropped whole.
    let dropped = cursor_line.dropped(scroll_x);
    let caret = (
        area.x
            .saturating_add(screen_u16(cursor_cell.saturating_sub(dropped).get())),
        (area.y + EDITOR_BAR_HEIGHT).saturating_add(screen_u16(editor.cursor_line - scroll_y)),
    );
    if !render_editor_dialog(f, tracker, editor, caret) {
        f.set_cursor_position(caret);
    }
}

fn render_mask_prompt(
    f: &mut Frame,
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
        f.set_cursor_position((super::describe::caret_column(x, prompt.after_caret()), y));
    }
}

/// Draw `text`, which ends at the caret, as a selection: Far shows an offered
/// mask selected, and that is what tells the user typing will replace it.
fn select_field_text(f: &mut Frame, text: &str, end: u16, y: u16) {
    let width = u16::try_from(Cells::of(text).get()).unwrap_or(u16::MAX);
    let cells = Rect::new(end.saturating_sub(width), y, width, 1).intersection(f.area());
    let selected = Style::default()
        .bg(palette::FIELD_SELECTED_BG)
        .fg(palette::FIELD_SELECTED_FG);
    f.buffer_mut().set_style(cells, selected);
}

fn render_quit_confirm(
    f: &mut Frame,
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

fn render_delete_confirm(
    f: &mut Frame,
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
    message.extend(
        files
            .iter()
            .take(DELETE_PREVIEW)
            .map(|path| super::entry_label(path)),
    );
    if files.len() > DELETE_PREVIEW {
        message.push(format!("… and {} more", files.len() - DELETE_PREVIEW));
    }

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

fn render_new_file_input(f: &mut Frame, tracker: &mut crate::mouse::DialogMouse, input: &str) {
    let input = super::tail_cells(input, Cells::columns(f.area().width.saturating_sub(9)));
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

fn render_editor_dialog(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    editor: &crate::models::Editor,
    caret: (u16, u16),
) -> bool {
    if let Some(choice) = editor.encoding_choice {
        render_encoding_dialog(f, tracker, choice);
    } else if let Some(prompt) = editor.search.prompt.as_ref() {
        if let Some((x, y)) = render_search_prompt(f, tracker, prompt) {
            f.set_cursor_position((x, y));
        }
    } else if let Some(needle) = editor.search.not_found.as_ref() {
        // Far reports a failed search rather than leaving the caret
        // still: an unmoved caret is what a key that did not register
        // looks like too.
        render_search_miss(f, tracker, needle);
    } else if let Some(choice) = editor.state.exit_choice() {
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
    } else if let Some(prompt) = editor.state.overwrite_prompt() {
        render_overwrite_prompt(f, tracker, prompt.answer);
    } else {
        return super::editor_lsp::popup(f, editor, caret);
    }
    true
}

/// A save found another program's write: say so before replacing it.
fn render_overwrite_prompt(
    f: &mut Frame,
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

/// Far's Make folder dialog (F7), and its Rename box (Shift+F6).
///
/// The same dialog, because the name field, the buttons and the cancelling are
/// the same. F7 adds the group Far puts under the rule: what kind of link to
/// make, what it points at, and whether the name field is a list. Rename has
/// none of that, so the rows are simply not drawn — unlike a greyed control,
/// which says "not for this answer", an absent one says "not for this key".
fn render_name_prompt(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &crate::models::NamePrompt,
) {
    use crate::models::{NameField, NamePurpose};

    let on = |field: NameField| prompt.field() == field;
    let question = match prompt.purpose() {
        NamePurpose::Rename => format!("Rename {} to:", prompt.original()),
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

/// Far's F5 Copy dialog: where to, with what rights, and what to do about
/// files already there.
///
/// The groups are Far's, separated by rules: the destination, then the access
/// rights, then everything about collisions, then the filter, then the
/// buttons. The order is not cosmetic — it runs from the answer the user
/// always supplies to the ones they almost never change.
fn render_copy_prompt(
    f: &mut Frame,
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

/// The question, the name, and whichever controls this purpose offers.
fn name_prompt_rows<'a>(
    prompt: &'a crate::models::NamePrompt,
    question: &'a str,
) -> Vec<dialog::Row<'a>> {
    use crate::models::NameField;
    let on = |field: NameField| prompt.field() == field;
    let mut rows = vec![
        dialog::Row::Label(question),
        dialog::Row::input(prompt.name(), on(NameField::Name), true),
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
        dialog::Row::Field {
            caption: "&Target:",
            value: prompt.link_target(),
            focused: on(NameField::Target),
            enabled: prompt.allows(NameField::Target),
        },
    ]
}

fn copy_rows<'a>(prompt: &'a crate::copy::CopyPrompt, caption: &'a str) -> Vec<dialog::Row<'a>> {
    use crate::copy::{AccessRights, CopyField};
    let on = |field| prompt.field == field;
    let mut rows = vec![
        dialog::Row::Label(caption),
        dialog::Row::input(&prompt.destination, on(CopyField::Destination), true),
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

fn render_editor_body(
    f: &mut Frame,
    editor: &crate::models::Editor,
    area: Rect,
    scroll_y: usize,
    scroll_x: Cells,
) {
    let body_area = Rect {
        y: area.y + EDITOR_BAR_HEIGHT,
        height: area.height.saturating_sub(EDITOR_BAR_HEIGHT),
        ..area
    };
    let body = editor_window(
        editor,
        scroll_y,
        usize::from(body_area.height),
        scroll_x,
        Cells::columns(body_area.width),
    );
    f.render_widget(Clear, body_area);
    f.render_widget(
        Paragraph::new(body).style(Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)),
        body_area,
    );
    super::editor_selection::render(f, editor, body_area, scroll_y, scroll_x);
    super::editor_lsp::diagnostics(f, editor, body_area, scroll_y, scroll_x);
    super::editor_brackets::render(f, editor, body_area, scroll_y, scroll_x);
}
fn render_encoding_dialog(
    f: &mut Frame,
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

fn render_search_miss(f: &mut Frame, tracker: &mut crate::mouse::DialogMouse, needle: &str) {
    dialog::render_warning(
        f,
        tracker,
        "Search",
        &[format!("\"{needle}\" not found")],
        &[dialog::Button::new("Ok").default_button().focused(true)],
    );
}
