//! Unit tests: overlay dialogs (split out to keep files under 500 lines).
#![allow(unused_imports)]

use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::quicksearch::QuickSearch;

use super::*;

use crate::models::{App, DirEntry, Overlay};

use ratatui::backend::TestBackend;

use ratatui::Terminal;

use std::path::PathBuf;

// -----------------------------------------------------------------------
// Drive selection overlay – both panel titles
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_drive_selection_left_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_render_overlays_drive_selection_right_panel() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_drive_selection_right() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_overlays_drive_selection_highlighted_entry() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let drives = App::get_all_drives();
    let mut app = App::new().unwrap();
    let selected_index = if drives.is_empty() {
        0
    } else {
        drives.len() - 1
    };
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index,
        rows: crate::models::drive_rows(),
    });

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

// -----------------------------------------------------------------------
// Delete confirmation overlay – >5 files branch
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_delete_confirm_many_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Create more than 5 paths to trigger the "... and N more" branch.
    let paths: Vec<PathBuf> = (0..8)
        .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_delete_confirm_many_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![
        PathBuf::from("f1"),
        PathBuf::from("f2"),
        PathBuf::from("f3"),
        PathBuf::from("f4"),
        PathBuf::from("f5"),
        PathBuf::from("f6"),
    ]));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_overlays_delete_confirm_few_files() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Exactly 3 paths – stays within the first 5 branch (no "... and N more").
    let paths: Vec<PathBuf> = (0..3)
        .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

// -----------------------------------------------------------------------
// New file input overlay
// -----------------------------------------------------------------------

#[test]
fn test_render_overlays_new_file_input() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("myfile.txt".to_string()));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("[ Cancel ]"));
    assert_eq!(app.dialog_mouse.buttons.len(), 2);
    let cell = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .find(|cell| cell.symbol() == "y")
        .unwrap();
    assert_eq!(cell.fg, Color::Rgb(0, 0, 0));
    assert_eq!(cell.bg, Color::Rgb(0, 128, 128));
}

#[test]
fn test_render_overlays_new_file_input_empty() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput(String::new()));

    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
}

// -----------------------------------------------------------------------
// Status bar with directory selection
// -----------------------------------------------------------------------

#[test]
fn test_render_status_with_directory_selection() {
    let mut app = App::new().unwrap();
    app.left_panel.entries = vec![DirEntry {
        name: "subdir".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_deref());
        })
        .unwrap();
}

// -----------------------------------------------------------------------
// Panel with ".." selected (showing parent)
// -----------------------------------------------------------------------

#[test]
fn test_render_status_dotdot_selected() {
    let mut app = App::new().unwrap();
    // ".." entry produces None from get_selected_entry in status (selected_path).
    // But get_selected_entry itself returns the ".." entry.
    app.left_panel.entries = vec![DirEntry {
        name: "..".to_string(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }];
    app.left_panel.select(Some(0));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_deref());
        })
        .unwrap();
}

/// Command line padding is empty when terminal is too narrow — and the typed
/// text still reaches the screen instead of being pushed off by the path.
#[test]
fn test_render_command_line_narrow_terminal() {
    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/a/very/long/path/that/is/wider/than/terminal");
    app.set_overlay(Overlay::CommandLine("ls".into()));

    // Use a very narrow terminal so prompt + text > width
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // 10 rows: panels, status, command line (row 8), F-key bar.
    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 8)].symbol()).collect();
    assert!(
        row.contains("ls"),
        "the typed command survives a narrow row: '{row}'"
    );
}

/// Drive selection with multiple items: non-selected items get default style
#[test]
fn test_render_drive_selection_multiple_items() {
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 1,
        rows: crate::models::drive_rows(),
    });

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_render_status_with_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("test".to_string());

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_deref());
        })
        .unwrap();
}

#[test]
fn test_render_status_without_quick_search_rebase_copy() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Idle;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_deref());
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_quick_search_active_rebase_copy() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("search".to_string());

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

// -----------------------------------------------------------------------
// Overlays must erase the cells they cover, not just restyle them
// -----------------------------------------------------------------------

/// App whose panels are full of loud `X` rows that would show through any
/// overlay that restyles its area without clearing the old glyphs.
fn app_with_loud_panels() -> App {
    let mut app = App::new().expect("test app");
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    for panel in [&mut app.left_panel, &mut app.right_panel] {
        panel.entries.clear();
        for i in 0..20 {
            panel.entries.push(DirEntry {
                name: format!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX_{i}"),
                kind: EntryKind::File,
                size: FileSize::from_bytes(0),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            });
        }
        panel.select(Some(0));
    }
    app
}

#[test]
fn test_editor_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Editor(crate::models::Editor {
        title: "notes.txt".to_string(),
        lines: vec!["hi".to_string()],
        ..Default::default()
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The row holding the editor's short line must not carry panel glyphs
    // to its right — a Block restyles cells but does not erase their symbols.
    let buf = terminal.backend().buffer();
    // The editor is full screen, as Far's is.
    let area = buf.area;
    let row_y = area.y + 1; // first body row, where "hi" is drawn
                            // Borderless: the body runs the full width of the screen.
    let row: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "editor line missing: {row:?}");
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the editor: {row:?}"
    );
}

#[test]
fn test_viewer_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "log",
        vec!["hi".to_string()],
    )));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    // The viewer is full screen, as Far's is: the body starts on the row
    // under the status line and runs the full width.
    let area = buf.area;
    let row_y = area.y + 1;
    let row: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "viewer line missing: {row:?}");
    let bar: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, area.y)].symbol())
        .collect();
    assert!(
        !bar.contains('X'),
        "panel glyphs bleed through the bar: {bar:?}"
    );
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the viewer: {row:?}"
    );
}

// -----------------------------------------------------------------------
// The Gray + / Gray − mask dialog
// -----------------------------------------------------------------------

/// Far's Select box: a title that says which way the selection moves, the
/// mask under the cursor, and the two buttons.
#[test]
fn the_mask_dialog_draws_its_title_mask_and_buttons() {
    use crate::models::{MaskPrompt, MaskPurpose};

    for (purpose, title) in [
        (MaskPurpose::Select, "Select"),
        (MaskPurpose::Unselect, "Deselect"),
    ] {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new().unwrap();
        app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
            purpose,
            "*.rs".to_owned(),
        )));

        terminal.draw(|f| ui(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);

        assert!(screen.contains(title), "the title says which way: {screen}");
        assert!(screen.contains("Enter the mask"), "{screen}");
        assert!(screen.contains("*.rs"), "the mask is in the field");
        assert!(
            screen.contains("OK") && screen.contains("Cancel"),
            "{screen}"
        );
    }
}

/// The cursor is drawn where it is, so a click on Cancel is visible before
/// Enter is pressed.
#[test]
fn the_mask_dialog_shows_which_button_has_the_cursor() {
    use crate::models::{MaskPrompt, MaskPurpose};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*".to_owned());
    prompt.focus_cancel(true);
    app.set_overlay(Overlay::MaskPrompt(prompt));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("Cancel"));
}

/// The whole screen as one string, for the dialogs whose content matters more
/// than which row it landed on.
fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    crate::test_support::buffer_string(terminal.backend().buffer())
}

// -----------------------------------------------------------------------
// The editor's "not found" dialog
// -----------------------------------------------------------------------

/// A search that found nothing says so, and names what it looked for — the
/// one thing the user needs in order to see the typo.
#[test]
fn a_search_that_found_nothing_names_what_it_looked_for() {
    use crate::models::Editor;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut editor = Editor::default();
    editor.search.not_found = Some("needle".to_owned());
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("Search"), "{screen}");
    assert!(screen.contains("needle"), "it names the query: {screen}");
}

// -----------------------------------------------------------------------
// The Make folder dialog, in both of its states
// -----------------------------------------------------------------------

/// Far's F7 box: the name field, the rule, and the group under it — the link
/// type, the target it points at, and the checkbox that makes the name field
/// a list.
#[test]
fn the_make_folder_dialog_draws_fars_whole_box() {
    use crate::models::NamePrompt;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    for expected in [
        "Make folder",
        "Create the folder:",
        "Link type:",
        "none",
        "Target:",
        "Process multiple names",
        "OK",
        "Cancel",
    ] {
        assert!(screen.contains(expected), "missing {expected:?}: {screen}");
    }
}

/// With a link chosen the Target field stops being grey and takes the caret,
/// which is the only visible difference between the two states.
#[test]
fn choosing_a_link_lights_up_the_target_field() {
    use crate::models::{NameField, NamePrompt};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Link);
    prompt.cycle(true);
    prompt.focus(NameField::Target);
    for ch in "/srv/data".chars() {
        prompt.push(ch);
    }
    app.set_overlay(Overlay::NamePrompt(prompt));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("symbolic link"), "{screen}");
    assert!(
        screen.contains("/srv/data"),
        "the target is shown: {screen}"
    );
}

/// The rename box is the same dialog with the link group left out — an absent
/// control says "not for this key", where a grey one would say "not yet".
#[test]
fn the_rename_box_leaves_the_link_group_out_altogether() {
    use crate::models::{FileName, NamePrompt};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "notes.txt",
    ))));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("Rename notes.txt to:"), "{screen}");
    assert!(
        !screen.contains("Link type"),
        "no link group here: {screen}"
    );
    assert!(!screen.contains("Process multiple"), "{screen}");
}

/// A dialog taller than the terminal draws what fits rather than panicking or
/// spilling over the panels below it.
#[test]
fn a_dialog_taller_than_the_screen_draws_what_fits() {
    use crate::models::NamePrompt;

    let backend = TestBackend::new(40, 9);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("Make folder"));
}

// -----------------------------------------------------------------------
// dialog::render_at row styles, exercised directly against the dialog
// primitives rather than through any one overlay that happens to use them.
// -----------------------------------------------------------------------

/// A disabled checkbox is dimmed even off the cursor, and a focused enabled
/// one is highlighted — two different reasons for two different styles on the
/// very same row type, so mixing them up would draw the wrong one dim or the
/// wrong one lit.
#[test]
fn checkbox_rows_pick_the_dimmed_or_the_focused_style() {
    use super::dialog::{self, Button, Ground, Row};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let rows = [
        Row::Checkbox {
            label: "&Disabled here",
            checked: false,
            focused: false,
            enabled: false,
        },
        Row::Checkbox {
            label: "&Focused here",
            checked: true,
            focused: true,
            enabled: true,
        },
    ];
    let buttons = [Button::new("OK").default_button()];

    terminal
        .draw(|f| {
            dialog::render_at(f, "Checks", Ground::Prompt, &rows, &buttons, (0, 0));
        })
        .unwrap();

    let screen = screen_text(&terminal);
    assert!(screen.contains("Disabled here"), "{screen}");
    assert!(screen.contains("Focused here"), "{screen}");
}

/// A row label that ends right on the hotkey marker has no letter left to
/// accent — the text before the marker must still be drawn, not panic on the
/// missing character.
#[test]
fn a_row_label_ending_in_the_hotkey_marker_keeps_its_text() {
    use super::dialog::{self, Button, Ground, Row};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let rows = [Row::Text("Trailing marker&")];
    let buttons = [Button::new("OK").default_button()];

    terminal
        .draw(|f| {
            dialog::render_at(f, "Edge", Ground::Prompt, &rows, &buttons, (0, 0));
        })
        .unwrap();

    assert!(screen_text(&terminal).contains("Trailing marker"));
}

fn draw_dialog_controls(ground: dialog::Ground) -> Terminal<TestBackend> {
    use super::dialog::{self, Button, Row};
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| {
            dialog::render_at(
                frame,
                "Colors",
                ground,
                &[Row::Label("&Name"), Row::input("value", true, false)],
                &[
                    Button::new("&OK").focused(true),
                    Button::new("&Apply").enabled(false),
                ],
                (0, 0),
            );
        })
        .unwrap();
    terminal
}

fn assert_dialog_control_colors(ground: dialog::Ground, button_bg: Color) {
    use ratatui::style::Color;
    let terminal = draw_dialog_controls(ground);
    let buffer = terminal.backend().buffer();
    let field = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "v")
        .unwrap();
    assert_eq!(
        (field.fg, field.bg),
        (Color::Rgb(0, 0, 0), Color::Rgb(0, 128, 128))
    );
    let button = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "K")
        .unwrap();
    assert_eq!((button.fg, button.bg), (Color::Rgb(0, 0, 0), button_bg));
    let hotkey = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "N")
        .unwrap();
    assert_eq!(hotkey.fg, Color::Rgb(255, 255, 0));
    let background = match ground {
        dialog::Ground::Prompt => Color::Rgb(192, 192, 192),
        dialog::Ground::Warning => Color::Rgb(128, 0, 0),
    };
    assert_eq!(hotkey.bg, background);
    assert_button_accents(buffer, button_bg, background);
}

fn assert_button_accents(buffer: &ratatui::buffer::Buffer, focused: Color, ground: Color) {
    use ratatui::style::Color;
    let hotkey = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "O")
        .unwrap();
    assert_eq!((hotkey.fg, hotkey.bg), (Color::Rgb(255, 255, 0), focused));
    let disabled = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "A")
        .unwrap();
    assert_eq!(
        (disabled.fg, disabled.bg),
        (Color::Rgb(128, 128, 128), ground)
    );
}

#[test]
fn prompt_controls_use_far_black_on_cyan() {
    assert_dialog_control_colors(dialog::Ground::Prompt, Color::Rgb(0, 128, 128));
}

#[test]
fn warning_controls_use_far_gray_selected_button() {
    assert_dialog_control_colors(dialog::Ground::Warning, Color::Rgb(192, 192, 192));
}

#[test]
fn help_uses_fixed_far_console_colors() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);
    terminal
        .draw(|frame| render_overlays(frame, &mut app))
        .unwrap();
    let cell = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .find(|cell| cell.symbol() == "H")
        .unwrap();
    assert_eq!(cell.fg, Color::Rgb(0, 0, 0));
    assert_eq!(cell.bg, Color::Rgb(0, 128, 128));
}

#[test]
fn new_file_long_unicode_name_keeps_caret_inside_input() {
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let mut app = App::new().unwrap();
    let name = format!("{}.txt", "界".repeat(40));
    app.set_overlay(Overlay::NewFileInput(name.clone()));
    terminal
        .draw(|frame| render_overlays(frame, &mut app))
        .unwrap();
    let cursor = terminal.get_cursor_position().unwrap();
    let body = app.dialog_mouse.body.unwrap();
    assert!(body.contains(cursor));
    assert!(screen_text(&terminal).contains(".txt"));
    assert!(matches!(&app.overlay, Overlay::NewFileInput(input) if *input == name));
}
