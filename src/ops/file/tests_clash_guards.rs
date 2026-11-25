//! Purpose: An answer to the "File already exists" question that comes with
//! no question open leaves the dialog that is open alone.

use super::tests_transfer_policies::app_between;
use super::*;
use crate::models::{ClashAnswer, Overlay};
use tempfile::TempDir;

#[test]
fn existing_file_actions_without_a_question_preserve_the_current_overlay() {
    let root = TempDir::new().unwrap();
    let mut app = app_between(root.path(), root.path());
    app.set_overlay(Overlay::Help);

    answer_existing(&mut app, ClashAnswer::KeepBoth);
    cancel_existing(&mut app);

    assert!(matches!(app.overlay, Overlay::Help));
}
