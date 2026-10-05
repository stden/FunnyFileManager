//! Purpose: Then steps about the files and directories in the workspace.

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
