//! Purpose: A text undo record must never write into the secret field.

use super::*;
use crate::kdbx::tests_support::{focus, unlocked_sample};

#[test]
fn a_text_record_cannot_overwrite_the_password() {
    let (_directory, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("open form");
    let form = session
        .vault_mut()
        .expect("unlocked")
        .form_mut()
        .expect("form");
    let before = form.password_masked();
    form.undo = Some(EditUndo {
        field: EditField::Password,
        value: EditValue::Text("ordinary visible text".to_owned()),
        dirty: false,
    });
    form.undo();
    assert_eq!(form.password_masked(), before);
}
