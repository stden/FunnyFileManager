//! Purpose: Scrolling and paging through a listing longer than the panel.

use super::*;

// ─── scrolling a long listing ───────────────────────────────────────────────

/// Only the visible rows are built now, so scrolling is worth driving through
/// the real event loop rather than trusting the arithmetic alone.
#[test]
fn e2e_scrolling_a_long_listing_follows_the_cursor() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..200 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    // At the top: the first files are on screen and the last are not.
    let top = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    assert!(top.contains("file-0000"), "{top}");
    assert!(!top.contains("file-0199"), "the end is not on screen yet");

    // End jumps to the last entry, which must now be visible.
    let bottom = run_until(left.path(), right.path(), vec![key(KeyCode::End)]);
    assert!(
        bottom.contains("file-0199"),
        "End must scroll the last entry into view:\n{bottom}"
    );
    assert!(!bottom.contains("file-0000"), "and the top is gone");

    // Home brings it back.
    let again = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::End), key(KeyCode::Home)],
    );
    assert!(again.contains("file-0000"), "{again}");
}

/// Paging down and back up puts the cursor where it started.
///
/// The cursor, not the view: the offset does not come back to zero — after
/// paging up onto the second entry the listing is drawn from there, with ".."
/// scrolled off above it. That is how it behaved before the window was made
/// lazy too, so it is asserted as it is rather than quietly changed here.
#[test]
fn e2e_paging_down_and_back_returns_the_cursor() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..200 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    let there_and_back = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key(KeyCode::PageDown),
            key(KeyCode::PageDown),
            key(KeyCode::PageUp),
            key(KeyCode::PageUp),
        ],
    );

    // The panel's info row names the entry under the cursor.
    assert!(
        there_and_back.contains("file-0000.txt"),
        "two pages down and two back is the entry it began on:\n{there_and_back}"
    );
}

/// Every row a listing has must be reachable one keypress at a time, with the
/// cursor always on screen — the property the window exists to guarantee.
#[test]
fn e2e_holding_down_walks_the_whole_listing() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..60 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    // Sixty Downs from "..": past the end, so it settles on the last entry.
    let events: Vec<Event> = std::iter::repeat_n(key(KeyCode::Down), 60).collect();
    let screen = run_until(left.path(), right.path(), events);
    assert!(
        screen.contains("file-0059"),
        "the last entry must be on screen:\n{screen}"
    );
}
