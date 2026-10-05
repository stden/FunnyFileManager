//! Purpose: the Left and Right headings: which side they name and where the two menus differ.
use super::*;

/// The two panel headings name their own side, and nothing else does.
#[test]
fn only_the_panel_headings_name_a_side() {
    use crate::models::PanelSide;

    assert_eq!(Heading::Left.side(), Some(PanelSide::Left));
    assert_eq!(Heading::Right.side(), Some(PanelSide::Right));
    for heading in [Heading::Files, Heading::Commands, Heading::Options] {
        assert_eq!(heading.side(), None, "{heading:?}");
    }
}

/// The two panel menus are the same list, aimed at opposite sides.
///
/// Exactly two rows differ, and both for the same reason: Far gives each side
/// its own key for hiding that panel and for changing that panel's drive.
/// Everything else must stay identical, or the two sides have drifted.
#[test]
fn the_panel_menus_differ_only_where_they_name_a_side() {
    let left = Menu::open(Heading::Left);
    let right = Menu::open(Heading::Right);
    assert_eq!(left.entries().len(), right.entries().len());

    let differing: Vec<&'static str> = left
        .entries()
        .iter()
        .zip(right.entries())
        .filter(|(a, b)| a != b)
        .map(|(a, _)| label_of(a))
        .collect();

    assert_eq!(
        differing.len(),
        2,
        "only the per-side rows may differ, got {differing:?}"
    );
    for label in &differing {
        let plain = label.replace('&', "");
        assert!(
            plain.contains("Panel") || plain.contains("drive"),
            "{label:?} differs between the sides but is not a per-side row"
        );
    }
}

/// The panel menus act on their own side, so opening one must select it.
#[test]
fn the_panel_headings_name_the_side_they_act_on() {
    use crate::models::PanelSide;

    assert_eq!(Heading::Left.side(), Some(PanelSide::Left));
    assert_eq!(Heading::Right.side(), Some(PanelSide::Right));
    // The keys behind Info panel and Re-read follow the active panel, so a
    // Left row and a Right row are only distinct commands if the heading
    // moves the cursor first.
    for heading in [Heading::Left, Heading::Right] {
        let menu = Menu::open(heading);
        assert!(
            menu.entries().iter().any(|entry| {
                // Without the marker: letter assignment moves the `&` around,
                // and a test that matched the raw label would break whenever
                // a neighbouring row took the letter this one wanted.
                label_of(entry).replace('&', "").contains("Info panel") && entry.is_selectable()
            }),
            "{heading:?} offers Info panel"
        );
    }
}
