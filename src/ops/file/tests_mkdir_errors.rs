//! Unit tests: `handle_mkdir`'s retry loop must only retry a name collision.
//!
//! `Err(e) if e.kind() == AlreadyExists => {}` is the one error the loop is
//! allowed to shrug off and retry past — a different failure (a parent
//! directory that no longer exists, permission denied, …) has to come back to
//! the caller instead of being silently retried 9999 times or swallowed.

use super::*;

use crate::models::PanelSide;

#[test]
fn mkdir_propagates_an_error_that_is_not_a_name_collision() {
    let mut app = App::new().expect("test setup");
    // A panel rooted at a path whose parent does not exist: `fs::create_dir`
    // fails with `NotFound`, never `AlreadyExists`, so the very first attempt
    // must return the error rather than looping through `new_folder_1`,
    // `new_folder_2`, ...
    app.left_panel.path = PathBuf::from("/definitely/does/not/exist/ffm-mkdir-error-test");
    app.active_panel = PanelSide::Left;

    let error = handle_mkdir(&mut app).expect_err("a missing parent must not be retried away");
    assert_ne!(
        error.kind(),
        io::ErrorKind::AlreadyExists,
        "the failure is a missing parent, not a name collision, and must not be misreported as one"
    );
}
