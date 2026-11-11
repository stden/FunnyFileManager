//! Purpose: Far-style ` Reading: N files ` banner geometry, colors and clipping.

use super::*;
use crate::progress::{DirectoryRead, PanelHeader};

fn text_of(buffer: &ratatui::buffer::Buffer) -> String {
    (buffer.area.left()..buffer.area.right())
        .map(|x| buffer[(x, buffer.area.y)].symbol())
        .collect()
}

#[test]
fn reading_banner_centers_teal_badge_inside_left_panel_top_border() {
    let screen = Rect::new(0, 0, 120, 30);
    let read = DirectoryRead {
        files: 629_371,
        header: Some(PanelHeader {
            x: 0,
            y: 1,
            width: 60,
        }),
    };
    let banner = reading_banner(screen, &read);
    assert_eq!(banner.area, Rect::new(1, 1, 58, 1));
    assert_eq!(
        text_of(&banner),
        "═════════════════ Reading: 629371 files ══════════════════"
    );

    let rule_cell = &banner[(1, 1)];
    assert_eq!(rule_cell.fg, palette::FRAME);
    assert_eq!(rule_cell.bg, FAR_BLUE);
    assert!(rule_cell.modifier.contains(Modifier::BOLD));

    let badge_cell = &banner[(20, 1)];
    assert_eq!(badge_cell.fg, palette::MENU_FG);
    assert_eq!(badge_cell.bg, palette::MENU_BG);
    assert!(!badge_cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn reading_banner_targets_right_panel_top_border_and_fallback_header() {
    let screen = Rect::new(0, 0, 120, 30);
    let right = reading_banner(
        screen,
        &DirectoryRead {
            files: 42,
            header: Some(PanelHeader {
                x: 60,
                y: 1,
                width: 60,
            }),
        },
    );
    assert_eq!(right.area, Rect::new(61, 1, 58, 1));
    assert!(text_of(&right).contains(" Reading: 42 files "));

    let fallback = reading_banner(
        screen,
        &DirectoryRead {
            files: 7,
            header: None,
        },
    );
    assert_eq!(fallback.area, Rect::new(1, 1, 58, 1));
    assert!(text_of(&fallback).contains(" Reading: 7 files "));
}

#[test]
fn reading_banner_clips_narrow_headers() {
    let narrow = reading_banner(
        Rect::new(0, 0, 80, 25),
        &DirectoryRead {
            files: 12345,
            header: Some(PanelHeader {
                x: 0,
                y: 1,
                width: 12,
            }),
        },
    );
    assert_eq!(narrow.area, Rect::new(1, 1, 10, 1));
    assert_eq!(text_of(&narrow), " Reading: ");
}

#[test]
fn reading_banner_skips_degenerate_areas() {
    let sample = DirectoryRead {
        files: 1,
        header: None,
    };
    assert_eq!(
        reading_banner(Rect::new(0, 0, 0, 10), &sample).area,
        Rect::default()
    );
    assert_eq!(
        reading_banner(Rect::new(0, 0, 10, 0), &sample).area,
        Rect::default()
    );
    let two_cell = DirectoryRead {
        files: 1,
        header: Some(PanelHeader {
            x: 0,
            y: 1,
            width: 2,
        }),
    };
    assert_eq!(
        reading_banner(Rect::new(0, 0, 80, 25), &two_cell).area,
        Rect::default()
    );
}
