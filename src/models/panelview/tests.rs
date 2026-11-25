use super::{PanelKind, PanelsShown};
use crate::models::PanelSide;
use crate::models::PanelView;

#[test]
fn sort_cycle_preserves_legacy_modes_and_returns_extended_modes_to_name() {
    use crate::models::SortMode::{
        AccessTime, CreationTime, Descriptions, Extension, ModificationTime, Name, Owner, Size,
        Unsorted,
    };

    for sequence in [
        &[Name, Extension, Size, Name][..],
        &[
            ModificationTime,
            Unsorted,
            CreationTime,
            AccessTime,
            Descriptions,
            Owner,
            Name,
        ][..],
    ] {
        for transition in sequence.windows(2) {
            assert_eq!(transition[0].next(), transition[1]);
        }
    }
}

#[test]
fn full_is_the_view_a_panel_starts_in() {
    assert_eq!(PanelView::default(), PanelView::Full);
    assert_eq!(PanelKind::default(), PanelKind::Files);
    assert_eq!(PanelsShown::default(), PanelsShown::Both);
}

/// Far marks exactly two of the ten modes fullscreen.
#[test]
fn only_detailed_and_long_descriptions_take_the_whole_screen() {
    let fullscreen: Vec<PanelView> = PanelView::ALL
        .into_iter()
        .filter(|view| view.is_fullscreen())
        .collect();
    assert_eq!(
        fullscreen,
        [PanelView::Detailed, PanelView::LongDescriptions]
    );
}

#[test]
fn every_view_has_its_own_label() {
    let mut labels: Vec<&str> = PanelView::ALL.iter().map(|view| view.label()).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), PanelView::ALL.len());
}

#[test]
fn hiding_one_panel_leaves_the_other_and_hiding_it_again_puts_both_back() {
    let one = PanelsShown::Both.toggled(PanelSide::Left);
    assert_eq!(one, PanelsShown::RightOnly);
    assert!(!one.shows(PanelSide::Left));
    assert!(one.shows(PanelSide::Right));
    assert_eq!(one.toggled(PanelSide::Left), PanelsShown::Both);
}

/// The last panel on screen cannot be hidden: the toggle is a no-op there.
#[test]
fn the_last_panel_stays() {
    assert_eq!(
        PanelsShown::LeftOnly.toggled(PanelSide::Left),
        PanelsShown::LeftOnly
    );
    assert_eq!(
        PanelsShown::RightOnly.toggled(PanelSide::Right),
        PanelsShown::RightOnly
    );
}

#[test]
fn the_panel_kinds_know_what_they_are() {
    assert!(PanelKind::Tree(Box::default()).is_tree());
    assert!(!PanelKind::Tree(Box::default()).is_quick_view());
    assert!(PanelKind::QuickView(Box::default()).is_quick_view());
    assert!(!PanelKind::Files.is_tree());
}

/// A fullscreen view on the active panel is drawn like a hidden other panel,
/// and read that way by the mouse too.
#[test]
fn a_fullscreen_active_panel_takes_the_whole_width() {
    assert_eq!(
        PanelsShown::Both.with_fullscreen(PanelSide::Right, true),
        PanelsShown::RightOnly
    );
    assert_eq!(
        PanelsShown::LeftOnly.with_fullscreen(PanelSide::Left, true),
        PanelsShown::LeftOnly
    );
    assert_eq!(
        PanelsShown::LeftOnly.with_fullscreen(PanelSide::Left, false),
        PanelsShown::LeftOnly,
        "a half-width view changes nothing"
    );
}

/// Only a `NetBox` panel takes a click on its own rows; every other kind
/// leaves the click to the file list's handling.
#[test]
fn only_a_netbox_panel_selects_its_own_clicked_row() {
    let mut store = crate::sessions::SessionStore::default();
    store.upsert(crate::test_support::netbox::session("a.test", None));
    let mut sessions =
        PanelKind::Sessions(Box::new(crate::models::SessionsPanel::from_store(&store)));
    assert_eq!(sessions.select_visible_row(1), Some(1));
    assert_eq!(sessions.select_visible_row(2), None);
    assert!(sessions.is_netbox());
    for mut other in [
        PanelKind::Files,
        PanelKind::Tree(Box::default()),
        PanelKind::QuickView(Box::default()),
    ] {
        assert_eq!(other.select_visible_row(0), None);
        assert!(!other.is_netbox());
    }
}
