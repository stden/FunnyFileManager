//! The panel title: a path too long for the top border is cut from its head,
//! and the cut is marked, so a cut path never reads as a relative one.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, Terminal};

use super::*;

/// The whole top border of a `width`-cell active panel over `path`.
fn top_border(path: &str, filtered: bool, width: u16) -> String {
    let mut panel = Panel::from_entries(PathBuf::from(path), Vec::new());
    panel.filter.hide_dotfiles = filtered;
    let mut terminal = Terminal::new(TestBackend::new(width, 8)).expect("terminal");
    terminal
        .draw(|frame| render_panel(frame, &mut panel, frame.area(), true, None))
        .expect("render panel");
    let buffer = terminal.backend().buffer();
    (0..width).map(|x| buffer[(x, 0)].symbol()).collect()
}

#[test]
fn a_long_path_is_cut_from_its_head_behind_an_ellipsis() {
    assert_eq!(
        top_border("/srv/x/agents/v-nav-8/work", false, 20),
        "╔ …ts/v-nav-8/work ╗"
    );
}

/// The filter's `*` comes out of the same budget as the path.
#[test]
fn a_filtered_long_path_keeps_the_star_and_the_ellipsis() {
    assert_eq!(
        top_border("/srv/x/agents/v-nav-8/work", true, 20),
        "╔ *…s/v-nav-8/work ╗"
    );
}

/// A path exactly as wide as the frame still needs room for the blanks
/// around it; without a cut of its own the head vanished unmarked.
#[test]
fn a_path_as_wide_as_the_frame_is_cut_and_marked() {
    assert_eq!(
        top_border("/srv/x/0123456789a", false, 20),
        "╔ …v/x/0123456789a ╗"
    );
}

#[test]
fn a_short_path_is_centred_whole() {
    assert_eq!(top_border("/srv/x", false, 20), "╔═════ /srv/x ═════╗");
}
