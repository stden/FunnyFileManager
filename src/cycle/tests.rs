use super::step;

const ORDER: [u8; 3] = [1, 2, 3];

#[test]
fn it_wraps_at_both_ends() {
    assert_eq!(step(&ORDER, 1, true), 2);
    assert_eq!(step(&ORDER, 3, true), 1);
    assert_eq!(step(&ORDER, 1, false), 3);
    assert_eq!(step(&ORDER, 3, false), 2);
}

#[test]
fn a_stranger_steps_to_the_first_rather_than_panicking() {
    assert_eq!(step(&ORDER, 9, true), 2, "treated as if it were the first");
    assert_eq!(step(&ORDER, 9, false), 3);
}

#[test]
fn an_empty_list_leaves_the_choice_alone() {
    assert_eq!(step::<u8>(&[], 7, true), 7);
}

/// Round the whole ring and back to the start, in both directions.
#[test]
fn a_full_turn_returns_to_where_it_began() {
    for forward in [true, false] {
        let mut at = ORDER[0];
        for _ in 0..ORDER.len() {
            at = step(&ORDER, at, forward);
        }
        assert_eq!(at, ORDER[0]);
    }
}
