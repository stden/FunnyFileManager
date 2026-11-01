//! Purpose: Given and Then steps about the files and directories in the workspace.

use cucumber::gherkin::Step;

use super::*;

#[then(expr = "the {word} file {string} contains {string}")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn file_contents(world: &mut TuiWorld, side: String, file: String, expected: String) {
    assert_eq!(
        fs::read_to_string(world.path(&side, &file)).expect("file"),
        expected
    );
}

#[then(expr = "the {word} file {string} does not exist")]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn absent(world: &mut TuiWorld, side: String, file: String) {
    assert!(!world.path(&side, &file).exists());
}

#[then(expr = "the {word} directory {string} exists")]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn directory(world: &mut TuiWorld, side: String, name: String) {
    assert!(world.path(&side, &name).is_dir());
}

/// A file whose text is the step's doc string, without the blank lines the
/// doc string keeps around its delimiters.
#[given(expr = "the {word} file {string} holds:")]
#[expect(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "a step written without its doc string must fail here; Cucumber dictates the world as `&mut`"
)]
fn holds(world: &mut TuiWorld, side: String, file: String, step: &Step) {
    let text = step.docstring.as_deref().expect("a doc string").trim();
    fs::write(world.path(&side, &file), text).expect("fixture file");
}

/// `count` lines named `line 01`, `line 02` and so on, long enough to fill
/// more than one screen when `count` is large.
#[given(expr = "the {word} file {string} has {int} numbered lines")]
#[expect(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "a Cucumber step fails by panicking; Cucumber dictates the world as `&mut`"
)]
fn numbered(world: &mut TuiWorld, side: String, file: String, count: usize) {
    let lines: Vec<String> = (1..=count).map(|n| format!("line {n:02}")).collect();
    fs::write(world.path(&side, &file), lines.join("\n")).expect("fixture file");
}

/// Ctrl+F types the file's own full path: the text is `prefix` and then that
/// path, which must name the same file.
#[then(expr = "the {word} file {string} is {string} followed by its full path")]
#[expect(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "a Cucumber step fails by panicking; Cucumber dictates the world as `&mut`"
)]
fn followed_by_full_path(world: &mut TuiWorld, side: String, file: String, prefix: String) {
    let path = world.path(&side, &file);
    let contents = fs::read_to_string(&path).expect("file");
    let typed = contents
        .strip_prefix(prefix.as_str())
        .expect("the text before the path");
    assert_eq!(
        fs::canonicalize(typed).expect("a path"),
        fs::canonicalize(&path).expect("the file")
    );
}
