//! Unit tests: the "Change drive" dialog on screen — centred and clamped,
//! its title and legend in the frame, and only the chosen row highlighted,
//! out to the box's edge.

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use super::table::body;
use super::tests::{disk, plugin, text};
use super::{centered, Widths};
use crate::diskfree::DriveKind;

/// The dialog takes its natural size, and never more than the screen.
#[test]
fn the_dialog_is_centred_and_clamped_to_the_screen() {
    let screen = Rect::new(0, 0, 80, 24);
    let area = centered(screen, 30, 6);
    assert_eq!(area.width, 30);
    assert_eq!(area.height, 6);
    assert_eq!(area.x, 25);
    assert_eq!(area.y, 9);

    let squeezed = centered(Rect::new(0, 0, 10, 3), 40, 20);
    assert_eq!((squeezed.width, squeezed.height), (10, 3));
    assert_eq!((squeezed.x, squeezed.y), (0, 0));
}

/// The whole dialog draws without panicking, and the title reaches the screen.
#[test]
fn the_dialog_draws_its_title_and_legend() {
    let rows = [disk("C:", DriveKind::Fixed, 100, 50)];
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    terminal
        .draw(|f| super::render(f, crate::models::PanelSide::Left, &rows, 0))
        .expect("draw");
    let screen: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(screen.contains("Change drive"), "the title is drawn");
    assert!(screen.contains("Enter"), "the legend is drawn");
}

/// The selection bar runs edge to edge even where the box is wider than the
/// table: the filler after the free column takes the row's background, not
/// the frame's. With the frame's ground there it stopped mid-row with a gap
/// whenever a plugin row or the footer widened the box.
#[test]
fn a_chosen_disk_row_is_highlighted_to_the_edge() {
    // The long plugin name makes interior() wider than table(), so the disk
    // rows have a filler to paint.
    let rows = [
        disk("C:", DriveKind::Fixed, 500_000_000_000, 52_000_000_000),
        plugin('0', "S3: a-rather-long-profile-name"),
    ];
    let widths = Widths::of(&rows);
    assert!(
        widths.interior() > widths.table(),
        "the fixture must leave a filler to paint"
    );

    let filler_backgrounds = |selected: usize| {
        body(&rows, &widths, selected)
            .into_iter()
            .map(|line| {
                line.spans
                    .iter()
                    .find(|span| {
                        !span.content.is_empty()
                            && span.content.chars().all(|ch| ch == ' ')
                            && span.content.len() > 2
                    })
                    .and_then(|span| span.style.bg)
            })
            .collect::<Vec<_>>()
    };
    // Line 1 is the disk row; chosen, its filler carries the selection bar.
    assert_eq!(
        filler_backgrounds(0)[1],
        Some(crate::ui::palette::DRIVE_SELECTED_BG)
    );
    // Not chosen, the filler falls back to the menu ground.
    assert_eq!(filler_backgrounds(2)[1], Some(crate::ui::palette::MENU_BG));
}

/// Past the tenth profile a row has no shortcut; it still renders, with a
/// blank hotkey cell so its name lines up with the keyed rows above.
#[test]
fn a_keyless_plugin_row_renders_a_blank_hotkey_cell() {
    let mut rows = vec![disk("C:", DriveKind::Fixed, 100, 50)];
    rows.extend((0..11).map(|n| plugin(char::from_digit(n % 10, 10).expect("digit"), "S3: p")));
    rows[11].key = None;
    let lines = text(&rows, 0);

    let width = lines[0].chars().count();
    for line in &lines {
        assert_eq!(line.chars().count(), width, "{line:?}");
    }
    let keyless = &lines[lines.len() - 2];
    assert!(keyless.starts_with("│   S3: p"), "{keyless:?}");
    let keyed = &lines[lines.len() - 3];
    assert!(keyed.starts_with("│ 9 S3: p"), "{keyed:?}");
}

/// The cursor row is the one that changes style; nothing else does.
#[test]
fn only_the_selected_row_is_highlighted() {
    let rows = [
        disk("C:", DriveKind::Fixed, 100, 50),
        disk("D:", DriveKind::Fixed, 100, 50),
    ];
    let widths = Widths::of(&rows);
    let highlighted = |selected: usize| {
        body(&rows, &widths, selected)
            .into_iter()
            .map(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(crate::ui::palette::HOTKEY))
            })
            .collect::<Vec<_>>()
    };
    // Line 0 is the top edge, so the drives are lines 1 and 2.
    assert_eq!(highlighted(0), vec![false, true, false, false]);
    assert_eq!(highlighted(1), vec![false, false, true, false]);
}
