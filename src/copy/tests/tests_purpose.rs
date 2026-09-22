//! Purpose: F5 and F6 in one dialog: what a move greys, which controls are wired, their rows.

use super::*;
use crate::copy::Purpose;

#[test]
fn the_two_purposes_name_themselves_differently() {
    assert_eq!(Purpose::Copy.title(), "Copy");
    assert_eq!(Purpose::Move.title(), "Rename/Move");
    assert_eq!(Purpose::Copy.question(), "Copy");
    assert_eq!(Purpose::Move.question(), "Rename or move");
    assert!(Purpose::Copy.verb().contains("Copy"));
    assert!(Purpose::Move.verb().contains("Rename"));
}

#[test]
fn the_caption_follows_the_purpose() {
    let moving = CopyPrompt::for_purpose(Purpose::Move, "\"a.txt\"", Path::new("/tmp"));
    assert_eq!(moving.caption(), "Rename or move \"a.txt\" t&o:");
}

/// A move has one destination and no filter. Far greys both rather than
/// hiding them, so the dialog keeps its shape between F5 and F6.
#[test]
fn a_move_greys_the_controls_that_do_not_apply() {
    let copying = CopyPrompt::for_purpose(Purpose::Copy, "x", Path::new("/tmp"));
    let moving = CopyPrompt::for_purpose(Purpose::Move, "x", Path::new("/tmp"));

    // Following a link is a copying choice; the other two are greyed on a
    // move for the same reason they are greyed everywhere — see
    // `controls_nothing_implements_cannot_be_ticked`.
    assert!(copying.allows(CopyField::SymlinkContents), "copy offers it");
    assert!(
        !moving.allows(CopyField::SymlinkContents),
        "a move must keep the link it was given"
    );
    for field in [
        CopyField::Destination,
        CopyField::Rights,
        CopyField::Existing,
        CopyField::Timestamps,
        CopyField::Copy,
        CopyField::Cancel,
    ] {
        assert!(moving.allows(field), "move still offers {field:?}");
    }
}

/// Tab must step over a greyed control: stopping the cursor on something that
/// does nothing undoes the point of greying it.
#[test]
fn tab_skips_the_greyed_controls_in_a_move() {
    let mut moving = CopyPrompt::for_purpose(Purpose::Move, "x", Path::new("/tmp"));
    moving.field = CopyField::Timestamps;
    moving.advance(true);
    assert_eq!(
        moving.field,
        CopyField::Copy,
        "the three greyed controls between them are stepped over"
    );
    moving.advance(false);
    assert_eq!(moving.field, CopyField::Timestamps, "and backwards too");
}

/// The space bar must not tick a box the dialog has greyed.
#[test]
fn space_does_not_toggle_a_greyed_checkbox() {
    let mut moving = CopyPrompt::for_purpose(Purpose::Move, "x", Path::new("/tmp"));
    moving.field = CopyField::Filter;
    moving.toggle();
    assert!(!moving.filter, "a greyed box stays as it is");
}

/// Every control a copy offers is reachable, and the two nothing implements
/// are not among them.
#[test]
fn tab_reaches_everything_a_copy_offers() {
    let mut copying = CopyPrompt::for_purpose(Purpose::Copy, "x", Path::new("/tmp"));
    let mut seen = vec![copying.field];
    for _ in 0..8 {
        copying.advance(true);
        seen.push(copying.field);
    }
    seen.sort_by_key(|field| format!("{field:?}"));
    seen.dedup();
    assert_eq!(seen.len(), 7, "reached: {seen:?}");
    assert!(seen.iter().all(|field| copying.allows(*field)));
}

/// The dialog's row order is shared between the renderer and the mouse, so a
/// click reaches the control the user is pointing at. This walks the whole
/// list: every control must have exactly one row, and no two the same.
#[test]
fn every_control_sits_on_its_own_row() {
    use crate::copy::field_at_row;

    let mut found: Vec<(usize, CopyField)> = (0..12)
        .filter_map(|row| field_at_row(row).map(|field| (row, field)))
        .collect();
    assert_eq!(found.len(), 7, "seven clickable controls: {found:?}");

    // The captions and the three rules are not controls.
    for blank in [0, 2, 4, 9, 11] {
        assert_eq!(field_at_row(blank), None, "row {blank}");
    }

    found.sort_by_key(|(_, field)| format!("{field:?}"));
    let fields: Vec<_> = found.iter().map(|(_, field)| *field).collect();
    let mut unique = fields.clone();
    unique.dedup();
    assert_eq!(unique.len(), fields.len(), "no control appears twice");

    // The buttons are not body rows; everything else is reachable.
    assert!(!fields.contains(&CopyField::Copy));
    assert!(!fields.contains(&CopyField::Cancel));
    assert_eq!(fields.len(), 7);
}

/// A box that ticks and quietly does nothing is worse than one that visibly
/// cannot: nothing behind "Process multiple destinations" or "Use filter"
/// carries them out, so the dialog refuses them and Tab walks past.
#[test]
fn controls_nothing_implements_cannot_be_ticked() {
    let mut prompt = CopyPrompt::new("marked", Path::new("/tmp"));

    for field in [CopyField::MultipleDestinations, CopyField::Filter] {
        assert!(!field.is_wired(), "{field:?} is drawn as if it worked");
        prompt.field = field;
        prompt.toggle();
    }
    assert!(!prompt.multiple_destinations);
    assert!(!prompt.filter);

    // Tab never stops on them.
    prompt.field = CopyField::SymlinkContents;
    prompt.advance(true);
    assert_ne!(prompt.field, CopyField::MultipleDestinations);
    assert_ne!(prompt.field, CopyField::Filter);
}

/// The link choice belongs to copying: a move greys it out, because following
/// a link there would delete what the user had and leave a copy of somewhere
/// else in its place.
#[test]
fn the_link_choice_is_a_copying_choice() {
    use crate::utils::LinkPolicy;

    let mut copy = CopyPrompt::for_purpose(Purpose::Copy, "marked", Path::new("/tmp"));
    assert_eq!(copy.link_policy(), LinkPolicy::Recreate, "off by default");
    copy.field = CopyField::SymlinkContents;
    copy.toggle();
    assert_eq!(copy.link_policy(), LinkPolicy::Follow);

    let mut move_prompt = CopyPrompt::for_purpose(Purpose::Move, "marked", Path::new("/tmp"));
    move_prompt.field = CopyField::SymlinkContents;
    move_prompt.toggle();
    assert_eq!(
        move_prompt.link_policy(),
        LinkPolicy::Recreate,
        "a move must keep the link it was given"
    );
}
