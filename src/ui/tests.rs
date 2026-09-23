//! Unit tests and shared test helpers for `ui`.
#![allow(unused_imports)]

use super::*;
use crate::lineedit::LineEditor;
use crate::models::EntryKind;
use crate::models::FileName;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::Recall;
use crate::models::ViewMode;
use crate::quicksearch::QuickSearch;
use crate::status::Status;

use crate::models::{App, Candidate, CandidateKind, CommandLine, DirEntry, Menu, Overlay};

use ratatui::backend::TestBackend;

use ratatui::Terminal;

use std::path::PathBuf;

/// A dropdown over `candidates` with row `index` highlighted.
///
/// `Menu` keeps its selection private so it can never point past the list;
/// walking to the row with `next` is how a test asks for one, and it goes
/// through the same code the `Tab` key does.
fn menu_selecting(candidates: Vec<Candidate>, index: usize) -> Option<Menu> {
    let mut menu = Menu::new(candidates)?;
    for _ in 0..index {
        menu.next();
    }
    Some(menu)
}

#[test]
fn test_panel_page_size_is_terminal_height_minus_chrome_and_borders() {
    // 24 rows - 3 chrome lines (menu bar, command line, F-key bar) - 2 panel
    // borders - 1 column-header row - 1 column rule - 1 info row = 16 entry
    // rows, which is exactly what `render_panel_contents` draws: a page must
    // move by what is on screen, or PgDn skips the first entry of every page.
    //
    // The status bar's line came back to the panels and was spent there: Far
    // puts a column header at the top and the current entry's name at the
    // bottom, and both live inside the frame.
    assert_eq!(panel_page_size(24), 16);
    assert_eq!(panel_page_size(50), 42);
}

#[test]
fn test_panel_page_size_never_returns_zero_on_tiny_terminals() {
    // A zero page size would make PgUp/PgDn silently do nothing.
    assert_eq!(panel_page_size(8), 1);
    assert_eq!(panel_page_size(6), 1);
    assert_eq!(panel_page_size(1), 1);
    assert_eq!(panel_page_size(0), 1);
}

#[test]
fn test_ui_render_with_right_panel_active() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_status_with_file_selection() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Add a file to the panel
    app.left_panel.entries.push(DirEntry {
        name: "test.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });
    app.left_panel
        .select(Some(app.left_panel.entries.len() - 1));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_status_no_selection() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Clear panel selection
    app.left_panel.select(None);

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_help_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_ui_render_with_viewer_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_string(), "Line 2".to_string()],
    )));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_status_right_panel_active_with_file() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;

    // Add a file to the right panel
    app.right_panel.entries.push(DirEntry {
        name: "document.pdf".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(2_048_576), // 2MB,
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });
    app.right_panel
        .select(Some(app.right_panel.entries.len() - 1));

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_panel_with_various_entries() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Add various types of entries
    app.left_panel.entries.clear();
    app.left_panel.entries.push(DirEntry {
        name: "..".to_string(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    });
    app.left_panel.entries.push(DirEntry {
        name: "documents".to_string(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    });
    app.left_panel.entries.push(DirEntry {
        name: "image.jpg".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1_536_000), // ~1.5MB,
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_centered_rect_various_sizes() {
    let base = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
    };

    // Test different percentages
    let rect1 = centered_rect(80, 60, base);
    assert_eq!(rect1.width, 80);
    assert_eq!(rect1.height, 60);

    let rect2 = centered_rect(100, 100, base);
    assert_eq!(rect2.width, 100);
    assert_eq!(rect2.height, 100);

    let rect3 = centered_rect(10, 10, base);
    assert_eq!(rect3.width, 10);
    assert_eq!(rect3.height, 10);
}

#[test]
fn test_centered_rect_small_base() {
    let base = Rect {
        x: 0,
        y: 0,
        width: 10,
        height: 10,
    };
    let rect = centered_rect(50, 50, base);
    assert_eq!(rect.width, 5);
    assert_eq!(rect.height, 5);
    assert_eq!(rect.x, 3);
    assert_eq!(rect.y, 3);
}

#[test]
fn test_ui_render_with_minimal_terminal_size() {
    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_ui_render_with_very_long_paths() {
    let mut app = App::new().unwrap();
    let long_path = "/very/long/path/that/exceeds/normal/display/width/in/terminal/window";
    app.left_panel.path = PathBuf::from(long_path);
    app.right_panel.path = PathBuf::from(long_path);
    app.set_status(Status::CommandOutput("x".repeat(200)));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

#[test]
fn test_render_panel_with_empty_entries() {
    let mut app = App::new().unwrap();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_panel(f, &mut app.left_panel, area, true, None);
        })
        .unwrap();
}

#[test]
fn test_render_overlays_with_all_types() {
    let mut app = App::new().unwrap();

    app.set_overlay(Overlay::Help);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();

    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test Viewer",
        vec!["Line 1".to_string(), "Line 2".to_string()],
    )));
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();

    app.set_overlay(Overlay::CommandLine("test command".into()));
    terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    app.close_overlay();
}

#[test]
fn test_render_status_with_extreme_values() {
    let mut app = App::new().unwrap();

    app.left_panel.entries = vec![
        DirEntry {
            name: "..".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
        DirEntry {
            name: "huge_file.dat".to_string(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(u64::MAX),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
    ];
    app.left_panel.select(Some(1));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let footer = panel_footer(&app);
    terminal
        .draw(|f| {
            render_panel(f, &mut app.left_panel, f.area(), true, footer.as_ref());
        })
        .unwrap();
}

#[test]
fn test_centered_rect_edge_cases() {
    let base = Rect::new(0, 0, 100, 50);

    let rect1 = centered_rect(0, 0, base);
    assert_eq!(rect1.width, 0);
    assert_eq!(rect1.height, 0);

    let rect2 = centered_rect(200, 100, base);
    assert_eq!(rect2.width, 100);
    assert_eq!(rect2.height, 50);

    let rect3 = centered_rect(1, 1, base);
    assert_eq!(rect3.width, 1);
    assert_eq!(rect3.height, 0);

    let tiny_base = Rect::new(0, 0, 1, 1);
    let rect4 = centered_rect(50, 50, tiny_base);
    assert_eq!(rect4.width, 1);
    assert_eq!(rect4.height, 1);
}

#[test]
fn test_render_command_bar_variations() {
    let backend = TestBackend::new(120, 3);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_command_bar(f, area);
        })
        .unwrap();

    let small_backend = TestBackend::new(40, 3);
    let mut small_terminal = Terminal::new(small_backend).unwrap();

    small_terminal
        .draw(|f| {
            let area = f.area();
            render_command_bar(f, area);
        })
        .unwrap();
}

#[test]
fn test_render_panel_with_marked_files() {
    let mut app = App::new().unwrap();

    for i in 0..10 {
        app.left_panel.entries.push(DirEntry {
            name: format!("file{i}.txt"),
            kind: EntryKind::File,
            size: FileSize::from_bytes(100 * i),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        });
    }

    app.left_panel.marked.insert(FileName::from("file2.txt"));
    app.left_panel.marked.insert(FileName::from("file5.txt"));
    app.left_panel.marked.insert(FileName::from("file8.txt"));
    app.left_panel.select(Some(5));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_panel(f, &mut app.left_panel, area, true, None);
        })
        .unwrap();
}

/// Quick search wins the info row: it is about what the user is typing now,
/// while the entry name only repeats the row already highlighted above it.
#[test]
fn test_panel_footer_prefers_quick_search() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Active("test".to_string());

    assert_eq!(
        panel_footer(&app).map(|message| message.to_string()),
        Some("Search: test_".to_owned())
    );
}

/// With nothing to say, the info row falls back to the current entry's name —
/// so `panel_footer` must report "nothing", not an empty string that would
/// blank the name.
#[test]
fn test_panel_footer_is_none_when_idle() {
    let mut app = App::new().unwrap();
    app.quick_search = QuickSearch::Idle;
    app.set_status(Status::Cleared);

    assert_eq!(panel_footer(&app), None);
}

#[test]
fn test_ui_render_with_quick_search_active() {
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
// Console mode rendering
// -----------------------------------------------------------------------

#[test]
fn test_ui_render_console_mode() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.view = ViewMode::Console;

    // Must not panic and must return early (not render panels)
    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

// -----------------------------------------------------------------------
// Completion dropdown rendering
// -----------------------------------------------------------------------

#[test]
fn test_open_menu_renders_its_candidates() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m"),
        menu: Menu::new(vec![
            Candidate {
                text: "mark".to_string(),
                kind: CandidateKind::Word,
                hint: None,
            },
            Candidate {
                text: "mcp".to_string(),
                kind: CandidateKind::Word,
                hint: None,
            },
        ]),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("mark"), "{screen}");
    assert!(screen.contains("mcp"), "{screen}");
}

#[test]
fn test_menu_renders_at_most_ten_rows() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    let candidates: Vec<Candidate> = (0..30)
        .map(|i| Candidate {
            text: format!("candidate{i:02}"),
            kind: CandidateKind::Word,
            hint: None,
        })
        .collect();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("c"),
        // Selection near the end must stay visible, so the window scrolls.
        menu: menu_selecting(candidates, 29),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(
        screen.contains("candidate29"),
        "selection must be visible: {screen}"
    );
    assert!(
        !screen.contains("candidate00"),
        "window should have scrolled: {screen}"
    );
}

#[test]
fn test_candidate_hint_is_rendered() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Pin the panel paths so the command-line prompt (`{path}> `) cannot
    // accidentally contain the hint text — this repo's own checkout path
    // starts with the same prefix as a plausible bookmark hint, which would
    // make the assertion pass without the menu ever rendering.
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("bookmark go "),
        menu: Menu::new(vec![Candidate {
            text: "3".to_string(),
            kind: CandidateKind::Word,
            hint: Some("/srv/FunnyFileManager".to_string()),
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("/srv/FunnyFileManager"), "{screen}");
}

#[test]
fn test_menu_on_small_terminal_keeps_selection_visible() {
    // 8 rows total: panels(5, Min(0)) + status(1) + cmdline(1) + fkeybar(1).
    // The command line sits at y = 6, leaving only 4 rows above
    // it, so after the menu's own 2 border rows only 2 content rows fit —
    // far fewer than the 10-candidate list below. The scroll window must be
    // sized from that clamped space, not from the uncapped MENU_MAX_ROWS, or
    // the selected row gets silently dropped off the truncated end.
    let backend = TestBackend::new(40, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    let candidates: Vec<Candidate> = (0..10)
        .map(|i| Candidate {
            text: format!("candidate{i:02}"),
            kind: CandidateKind::Word,
            hint: None,
        })
        .collect();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("c"),
        menu: menu_selecting(candidates, 9),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(
        screen.contains("candidate09"),
        "selected candidate must stay visible even when the popup is \
         clamped by a small terminal: {screen}"
    );
    assert!(
        !screen.contains("candidate00"),
        "window should have scrolled past the start: {screen}"
    );
}

/// The history ghost renders as dimmed text right after the typed buffer
/// (and its cursor), so the suggestion reads as a continuation of what the
/// user typed.
#[test]
fn test_ghost_text_renders_after_the_buffer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("cd"),
        menu: None,
        ghost: Some(" /somewhere".to_string()),
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("cd /somewhere"), "{screen}");
}

/// With no ghost set, the row must render exactly as before this feature
/// existed — no stray suggestion text leaks onto the line.
#[test]
fn test_no_ghost_renders_nothing_extra() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine("cd".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(!screen.contains("cd /somewhere"), "{screen}");
}

/// A ghost far longer than the terminal is wide must still be clamped so the
/// command-line row never runs past the terminal edge (and the draw itself
/// must not panic on the arithmetic).
#[test]
fn test_long_ghost_does_not_overflow_the_row() {
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("cd"),
        menu: None,
        ghost: Some("x".repeat(40)),
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // panels(7, Min(0)) + status(1) puts the command line at row 8.
    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 8)].symbol()).collect();
    // Derived from the clamp: with active path "/test" (5 chars) and
    // reserved = cmd_text(2) + cursor(1) + ghost(40) + PROMPT_SUFFIX(2) = 45
    // against width 20, `tail_chars` collapses the path to "", so
    // prompt = "> " (2 chars). prefix_used = prompt(2) + cmd_text(2) +
    // cursor(1) = 5, leaving width.saturating_sub(5) = 15 columns for the
    // ghost. The caret sits on the ghost's first character (so the suggestion
    // reads as one continuous word), and the remaining 14 columns hold the
    // rest of it: "> " + "cd" + 16*'x' — exactly 20 chars, no padding left
    // over. Only 16 of the 40 'x's are shown.
    //
    // A `row.chars().count() == 20` assertion here would be tautological:
    // ratatui always fills every buffer cell for the full terminal width
    // regardless of what was actually drawn, so that count is 20 for any
    // output, including a broken clamp. Asserting the exact content is what
    // actually exercises the clamp math.
    // The prompt yields its columns to the typed text, so on a narrow row
    // only the sigil is left of it.
    assert!(
        row.ends_with("cdxxxxxxxxxxxxxxxxx") || row.ends_with("cdxxxxxxxxxxxxxxxx"),
        "command line row: '{row}'"
    );
}

/// When the horizontal scroll asks for more cells than the line holds, the
/// whole line is dropped and the caller gets an empty remainder (plus how many
/// cells the line actually spent).
#[test]
fn skip_cells_past_the_end_of_a_line_yields_empty_suffix() {
    assert_eq!(skip_cells("ab", Cells::new(10)), ("", Cells::new(2)));
    assert_eq!(skip_cells("", Cells::new(5)), ("", Cells::new(0)));
    assert_eq!(skip_cells("文档", Cells::new(10)), ("", Cells::new(4))); // two double-width glyphs
}

/// On a terminal with no room above the command line, the menu is skipped
/// rather than drawing a zero-height popup.
#[test]
fn completion_menu_on_zero_height_terminal_is_skipped() {
    // Tiny: chrome leaves no rows above the command line for the menu.
    let backend = TestBackend::new(40, 4);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/t");
    app.right_panel.path = PathBuf::from("/u");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m"),
        menu: Menu::new(vec![Candidate {
            text: "mark".to_string(),
            kind: CandidateKind::Word,
            hint: None,
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}

/// The screen column where `text` starts on row `y`, or None.
#[cfg(not(windows))]
fn column_of(buf: &ratatui::buffer::Buffer, y: u16, text: &str) -> Option<u16> {
    let row: String = (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect();
    row.find(text)
        .map(|byte| u16::try_from(row[..byte].chars().count()).expect("column fits u16"))
}

/// A leading dot is a Unix naming convention, not a hidden attribute, so a
/// `.dotdir` is painted like any other directory: white, as Far paints it.
/// Only Windows has a hidden bit to dim for.
#[cfg(not(windows))]
#[test]
fn test_a_dot_directory_is_painted_as_a_directory_not_dimmed() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join(".dotdir")).expect("mkdir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let buf = terminal.backend().buffer();

    let (x, y) = (0..24)
        .find_map(|y| column_of(buf, y, ".dotdir").map(|x| (x, y)))
        .expect("the entry is on screen");
    assert_eq!(buf[(x, y)].fg, palette::DIRECTORY);
}
