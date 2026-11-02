//! Purpose: unit tests for the PID query's limits, the Priority form's value and
//! focus, and the text bars.

use super::*;

#[test]
fn a_pid_query_takes_ten_digits_and_nothing_else() {
    let mut query = PidQuery::default();
    for ch in "42x7".chars() {
        query.push(ch);
    }
    assert_eq!(query.digits(), "427");
    assert_eq!(query.pid().map(Pid::get), Some(427));
    for _ in 0..20 {
        query.push('9');
    }
    assert_eq!(query.digits().len(), PidQuery::MAX_DIGITS);
    assert_eq!(query.pid(), None, "4 279 999 999 is above i32::MAX");
    query.pop();
    assert_eq!(query.digits(), "427999999");
}

#[test]
fn a_pid_query_never_names_a_pid_above_i32_max() {
    let spell = |digits: &str| {
        let mut query = PidQuery::default();
        digits.chars().for_each(|digit| query.push(digit));
        query.pid().map(Pid::get)
    };
    assert_eq!(spell("2147483647"), Some(2_147_483_647));
    assert_eq!(spell("2147483648"), None);
    assert_eq!(spell("0"), None);
    assert_eq!(spell(""), None);
}

#[test]
fn the_priority_form_steps_within_the_nice_range() {
    let targets = listing(vec![process(2, "vim")]).freeze_targets(Operation::Renice);
    let mut form = PriorityForm::new(targets, nice(18));
    assert_eq!(form.value().text(), "18");
    assert_eq!(form.targets().targets().len(), 1);
    form.step(NiceStep::Lower);
    form.step(NiceStep::Lower);
    assert_eq!(form.niceness(), Some(Niceness::MAX));
    form.value_mut().set_text(" -20 ");
    form.step(NiceStep::Raise);
    assert_eq!(form.niceness(), Some(Niceness::MIN));
    form.value_mut().set_text("-21");
    assert_eq!(form.niceness(), None, "Set is disabled");
    form.step(NiceStep::Lower);
    assert_eq!(
        form.value().text(),
        "-21",
        "a value that is not one is left to fix"
    );
}

#[test]
fn the_priority_form_focus_cycles_both_ways() {
    let mut form = PriorityForm::new(FrozenTargets::default(), nice(0));
    assert_eq!(form.focus(), FormFocus::Field);
    for expected in [FormFocus::Set, FormFocus::Cancel, FormFocus::Field] {
        form.focus_next();
        assert_eq!(form.focus(), expected);
    }
    for expected in [FormFocus::Cancel, FormFocus::Set, FormFocus::Field] {
        form.focus_previous();
        assert_eq!(form.focus(), expected);
    }
}

#[test]
fn the_bars_keep_their_text_and_the_search_its_result() {
    let mut filter = TextBar::new("fire");
    filter.line_mut().insert_char('|');
    assert_eq!(filter.line().text(), "fire|");
    let mut search = SearchBar::default();
    search.line_mut().insert_str("pyth");
    assert_eq!(search.found(), Found::Match);
    search.set_found(Found::NoMatch);
    assert_eq!(
        (search.line().text(), search.found()),
        ("pyth", Found::NoMatch)
    );
    assert_eq!(Modal::default(), Modal::Browse);
}
