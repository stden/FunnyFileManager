//! Purpose: the two commands the caret's rows are written with — the same
//! move, without the selection and with it.

use super::*;

#[test]
fn a_move_ends_the_selection_and_a_selecting_move_extends_it() {
    assert_eq!(
        go(To::PageUp),
        EditorCommand::Move {
            motion: To::PageUp,
            selection: SelectionMode::Move,
        }
    );
    assert_eq!(
        select(To::PageUp),
        EditorCommand::Move {
            motion: To::PageUp,
            selection: SelectionMode::Extend,
        }
    );
}

#[test]
fn every_selecting_row_is_its_moving_row_with_shift() {
    assert_eq!(MOVING.rows.len(), SELECTING.rows.len());
    for (moving, selecting) in MOVING.rows.iter().zip(SELECTING.rows) {
        let shifted: Vec<_> = moving.defaults.iter().copied().map(shift).collect();
        assert_eq!(selecting.defaults, shifted, "{}", selecting.name);
        let (
            EditorCommand::Move { motion, .. },
            EditorCommand::Move {
                motion: selected, ..
            },
        ) = (moving.command, selecting.command)
        else {
            panic!("{} or {} does not move", moving.name, selecting.name);
        };
        assert_eq!(motion, selected, "{}", selecting.name);
        assert_eq!(moving.command, go(motion));
        assert_eq!(selecting.command, select(motion));
    }
}
