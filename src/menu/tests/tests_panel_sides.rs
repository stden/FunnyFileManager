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

/// Every row of a panel menu, label and shortcut, separators as "-", as Far
/// prints them.
fn printed(heading: Heading) -> Vec<(String, String)> {
    Menu::open(heading)
        .entries()
        .iter()
        .map(|entry| match entry {
            Entry::Item {
                label, shortcut, ..
            } => ((*label).to_owned(), (*shortcut).to_owned()),
            Entry::Separator => ("-".to_owned(), String::new()),
        })
        .collect()
}

/// The rows both panel menus print, with the side's own toggle and drive keys.
fn expected(toggle: &str, drive: &str) -> Vec<(String, String)> {
    [
        ("&Brief", "Ctrl+1"),
        ("&Medium", "Ctrl+2"),
        ("&Full", "Ctrl+3"),
        ("&Wide", "Ctrl+4"),
        ("Detai&led", "Ctrl+5"),
        ("&Descriptions", "Ctrl+6"),
        ("Lon&g descriptions", "Ctrl+7"),
        ("Fil&e owners", "Ctrl+8"),
        ("File lin&ks", "Ctrl+9"),
        ("&Alternative full", "Ctrl+0"),
        ("-", ""),
        ("&Info panel", "Ctrl+L"),
        ("&Tree panel", "Ctrl+T"),
        ("Quick &view", "Ctrl+Q"),
        ("-", ""),
        ("&Sort modes", "Ctrl+F12"),
        ("Show lo&ng names", "Ctrl+N"),
        ("Panel &On/Off", toggle),
        ("&Re-read", "Ctrl+R"),
        ("&Change drive", drive),
    ]
    .into_iter()
    .map(|(label, shortcut)| (label.to_owned(), shortcut.to_owned()))
    .collect()
}

/// Every row of the two panel menus, label and shortcut, separators included,
/// as Far prints them. The menus are one table written by a macro for each
/// side, so a label that the macro gets wrong is wrong on both sides at once;
/// comparing the sides with each other cannot see that, so this pins the text.
#[test]
fn the_panel_menus_print_every_row_as_far_does() {
    assert_eq!(printed(Heading::Left), expected("Ctrl+F1", "Alt+F1"));
    assert_eq!(printed(Heading::Right), expected("Ctrl+F2", "Alt+F2"));
}
