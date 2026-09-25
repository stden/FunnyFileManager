//! Far's selection summary: with entries marked, the rule above the info row
//! says how many there are and how many bytes they hold, and it follows every
//! mark and unmark.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::models::{EntryKind, FileName, FileSize};

use super::*;
use crate::models::DirEntry;

/// A 42x8 panel: the top border, the header, three body rows, then the rule.
const RULE_ROW: u16 = 5;

fn entry(name: &str, bytes: u64) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        size: FileSize::from_bytes(bytes),
        ..DirEntry::default()
    }
}

fn listing() -> Panel {
    let folder = DirEntry {
        kind: EntryKind::Directory,
        ..entry("docs", 4096)
    };
    let mut panel = Panel::from_entries(
        PathBuf::from("/listing"),
        vec![
            folder,
            entry("crlf.txt", 29),
            entry("empty.txt", 0),
            entry("medium.txt", 1240),
            entry("sparse-5g.img", 5_368_709_120),
        ],
    );
    panel.select(Some(0));
    panel
}

fn mark(panel: &mut Panel, names: &[&str]) {
    panel.marked = names.iter().map(|name| FileName::from(*name)).collect();
}

fn draw(panel: &mut Panel) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(42, 8)).expect("test terminal");
    terminal
        .draw(|frame| render_panel(frame, panel, frame.area(), true, None))
        .expect("render panel");
    terminal.backend().buffer().clone()
}

fn rule(buffer: &Buffer) -> String {
    (0..42).map(|x| buffer[(x, RULE_ROW)].symbol()).collect()
}

#[test]
fn marked_files_are_counted_and_summed_on_the_rule() {
    let mut panel = listing();
    mark(&mut panel, &["crlf.txt", "empty.txt"]);
    let rule = rule(&draw(&mut panel));
    assert!(rule.contains(" 29 bytes in 2 files "), "{rule}");
}

/// Every mark and unmark redraws the summary; the byte count is grouped the
/// way Far prints it, and one file is one file.
#[test]
fn the_summary_follows_the_marks_and_groups_the_digits() {
    let mut panel = listing();
    mark(&mut panel, &["sparse-5g.img"]);
    let rule_text = rule(&draw(&mut panel));
    assert!(
        rule_text.contains(" 5,368,709,120 bytes in 1 file "),
        "{rule_text}"
    );

    mark(&mut panel, &["crlf.txt", "empty.txt", "medium.txt"]);
    let rule_text = rule(&draw(&mut panel));
    assert!(
        rule_text.contains(" 1,269 bytes in 3 files "),
        "{rule_text}"
    );
}

/// A folder's size is not in its row, so it is counted, not summed.
#[test]
fn a_marked_folder_is_counted_as_a_folder() {
    let mut panel = listing();
    mark(&mut panel, &["docs", "crlf.txt"]);
    let rule = rule(&draw(&mut panel));
    assert!(rule.contains(" 29 bytes in 1 file, 1 folder "), "{rule}");
}

#[test]
fn with_nothing_marked_the_rule_is_only_a_rule() {
    let mut panel = listing();
    let rule = rule(&draw(&mut panel));
    assert!(
        rule.chars().all(|ch| matches!(ch, '╟' | '─' | '┴' | '╢')),
        "{rule}"
    );
}

/// Far draws the summary in the marked entries' own colour.
#[test]
fn the_summary_is_drawn_in_the_marked_colour() {
    let mut panel = listing();
    mark(&mut panel, &["crlf.txt"]);
    let buffer = draw(&mut panel);
    let rule_text = rule(&buffer);
    let (before, _) = rule_text.split_once("29 bytes").expect("summary drawn");
    let x = u16::try_from(before.chars().count()).expect("column");
    assert_eq!(buffer[(x, RULE_ROW)].fg, palette::MARKED);
}

/// A summary wider than the rule falls back to Far's short sizes rather than
/// running into the frame.
#[test]
fn a_narrow_panel_shortens_the_summary_and_keeps_the_frame() {
    let mut panel = listing();
    mark(&mut panel, &["sparse-5g.img", "crlf.txt"]);
    let mut terminal = Terminal::new(TestBackend::new(24, 8)).expect("test terminal");
    terminal
        .draw(|frame| render_panel(frame, &mut panel, frame.area(), true, None))
        .expect("render panel");
    let buffer = terminal.backend().buffer();
    let rule: String = (0..24).map(|x| buffer[(x, RULE_ROW)].symbol()).collect();
    assert!(rule.contains(" 5.0 G in 2 files "), "{rule}");
    assert!(rule.starts_with('╟') && rule.ends_with('╢'), "{rule}");
}
