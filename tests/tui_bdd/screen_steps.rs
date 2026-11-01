//! Purpose: Then steps about the screen, the clipboard and how the TUI ended.

use super::*;

#[then(expr = "the clipboard contains {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn clipboard(world: &mut TuiWorld, text: String) {
    assert_eq!(world.ui().clipboard(), text);
}

#[then(expr = "the screen contains {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn contains(world: &mut TuiWorld, text: String) {
    let screen = world.ui().text();
    assert!(screen.contains(&text), "missing {text:?}\n{screen}");
}

/// The selection on screen: exactly this text, painted in the selection colour.
#[then(expr = "the screen highlights {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn highlights(world: &mut TuiWorld, text: String) {
    let highlighted = world.ui().highlighted();
    assert_eq!(highlighted, text, "highlighted\n{}", world.ui().text());
}

/// Either of two texts: the 8.3 short name of a long file name exists only on
/// Windows, so the same command line can be printed in two correct ways.
#[then(expr = "the screen contains {string} or {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn contains_either(world: &mut TuiWorld, first: String, second: String) {
    let screen = world.ui().text();
    assert!(
        screen.contains(&first) || screen.contains(&second),
        "missing {first:?} and {second:?}\n{screen}"
    );
}

/// `first`, and `after` later on the same row: a message that names a file,
/// rather than the name turning up somewhere else on the screen.
#[then(expr = "one screen line contains {string} and then {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn contains_in_order(world: &mut TuiWorld, first: String, after: String) {
    let screen = world.ui().text();
    let found = screen.lines().any(|line| {
        line.split_once(first.as_str())
            .is_some_and(|(_, rest)| rest.contains(&after))
    });
    assert!(found, "no line with {first:?} and then {after:?}\n{screen}");
}

#[then(expr = "the screen does not contain {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn does_not_contain(world: &mut TuiWorld, text: String) {
    let screen = world.ui().text();
    assert!(!screen.contains(&text), "unexpected {text:?}\n{screen}");
}

#[then(expr = "the {word} panel shows {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn panel_contains(world: &mut TuiWorld, side: String, text: String) {
    let screen = world.ui().panel_text(&side);
    let rows = usize::from(world.ui().size().height.saturating_sub(8));
    let found = screen.lines().skip(3).take(rows).any(|row| {
        row.contains(&text)
            || text.rsplit_once('.').is_some_and(|(stem, extension)| {
                row.split_once(stem)
                    .is_some_and(|(_, tail)| tail.trim_start().starts_with(extension))
            })
    });
    assert!(found, "missing {text:?} in {side}\n{screen}");
}

#[then(expr = "the screen size is {int} by {int}")]
fn size(world: &mut TuiWorld, width: u16, height: u16) {
    assert_eq!(world.ui().size(), Size::new(width, height));
}

#[then("the TUI exits successfully")]
fn exited(world: &mut TuiWorld) {
    world.ui().assert_exited();
}
