//! Purpose: Tests for priority values: the nice range and its steps, the kernel
//! priority htop prints as `RT`, and classes the viewer may set.

use super::{KernelPriority, NiceOutOfRange, NiceStep, Niceness, PriorityClass, SettableClass};

fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a nice value")
}

#[test]
fn niceness_is_minus_twenty_through_nineteen() {
    let cases = [
        (i32::MIN, None),
        (-21, None),
        (-20, Some(-20)),
        (0, Some(0)),
        (19, Some(19)),
        (20, None),
        (128, None),
        (i32::MAX, None),
    ];
    for (value, expected) in cases {
        assert_eq!(
            Niceness::try_from(value).ok().map(Niceness::get),
            expected,
            "{value}"
        );
    }
    assert_eq!(Niceness::MIN, nice(-20));
    assert_eq!(Niceness::MAX, nice(19));
    assert_eq!(Niceness::NORMAL, nice(0));
}

#[test]
fn an_out_of_range_nice_value_says_what_the_range_is() {
    assert_eq!(Niceness::try_from(20), Err(NiceOutOfRange));
    assert_eq!(
        NiceOutOfRange.to_string(),
        "A nice value is a whole number from -20 to 19."
    );
}

#[test]
fn a_nice_step_moves_by_one_and_stops_at_the_bounds() {
    let cases = [
        (0, NiceStep::Raise, Some(-1)),
        (0, NiceStep::Lower, Some(1)),
        (-19, NiceStep::Raise, Some(-20)),
        (-20, NiceStep::Raise, None),
        (-20, NiceStep::Lower, Some(-19)),
        (18, NiceStep::Lower, Some(19)),
        (19, NiceStep::Lower, None),
        (19, NiceStep::Raise, Some(18)),
    ];
    for (from, step, expected) in cases {
        assert_eq!(
            nice(from).step(step).map(Niceness::get),
            expected,
            "{from} {step:?}"
        );
    }
}

#[test]
fn the_top_realtime_priority_and_beyond_show_as_realtime() {
    let cases = [
        (-101, true),
        (-100, true),
        (-99, false),
        (-2, false),
        (0, false),
        (20, false),
        (39, false),
    ];
    for (raw, expected) in cases {
        let priority = KernelPriority::new(raw);
        assert_eq!(priority.is_realtime(), expected, "{raw}");
        assert_eq!(priority.get(), raw);
    }
}

#[test]
fn every_reported_class_but_realtime_is_settable() {
    let cases = [
        (PriorityClass::Idle, Some(SettableClass::Idle)),
        (PriorityClass::BelowNormal, Some(SettableClass::BelowNormal)),
        (PriorityClass::Normal, Some(SettableClass::Normal)),
        (PriorityClass::AboveNormal, Some(SettableClass::AboveNormal)),
        (PriorityClass::High, Some(SettableClass::High)),
        (PriorityClass::Realtime, None),
    ];
    for (class, settable) in cases {
        assert_eq!(class.settable(), settable, "{class:?}");
        if let Some(s) = settable {
            assert_eq!(s.as_class(), class);
        }
    }
}

#[test]
fn a_class_step_walks_idle_through_high_and_stops_at_both_ends() {
    let ascending = [
        SettableClass::Idle,
        SettableClass::BelowNormal,
        SettableClass::Normal,
        SettableClass::AboveNormal,
        SettableClass::High,
    ];
    for pair in ascending.windows(2) {
        let [lower, higher] = [pair[0], pair[1]];
        assert_eq!(lower.step(NiceStep::Raise), Some(higher), "{lower:?} up");
        assert_eq!(higher.step(NiceStep::Lower), Some(lower), "{higher:?} down");
    }
    assert_eq!(SettableClass::Idle.step(NiceStep::Lower), None);
    assert_eq!(SettableClass::High.step(NiceStep::Raise), None);
}
