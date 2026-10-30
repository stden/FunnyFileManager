//! Edit-form tests: keystrokes land in the focused field — never a newline,
//! and never the filter — and one-step undo.

use super::*;

#[test]
fn typing_goes_to_the_focused_field_and_sets_dirty() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");

    session.type_char('x', now);
    {
        let vault = session.vault().expect("vault is unlocked");
        let form = vault.form().expect("form is open");
        assert_eq!(form.title(), "GitHubx");
        assert!(form.is_dirty());
        assert_eq!(
            vault.filter(),
            "GitHub",
            "the keystroke went to the form, not the filter"
        );
    }

    session.backspace(now);
    assert_eq!(open_form(&session).title(), "GitHub");
    assert!(
        open_form(&session).is_dirty(),
        "an undone edit is still an edit"
    );

    open_form_mut(&mut session).focus_next();
    assert_eq!(open_form(&session).focus(), EditField::Username);
    session.type_char('!', now);
    assert_eq!(open_form(&session).username(), "alice!");
}

#[test]
fn password_keystrokes_stay_in_the_focused_secret_field() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    open_form_mut(&mut session).focus_next();
    // Two steps back from Username wraps Title -> Password: a ring, like the
    // name dialog's tab order.
    open_form_mut(&mut session).focus_prev();
    open_form_mut(&mut session).focus_prev();
    assert_eq!(open_form(&session).focus(), EditField::Password);
    session.type_char('z', now);
    assert_eq!(open_form(&session).password_masked(), "•".repeat(6));
    session.backspace(now);
    assert_eq!(open_form(&session).password_masked(), "•".repeat(5));
    assert_eq!(
        open_form(&session).title(),
        "GitHub",
        "password keystrokes stay out of the text fields"
    );
}

#[test]
fn zz_probe_undo_without_record_returns_false() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    assert!(!open_form_mut(&mut session).undo());
}

#[test]
fn zz_probe_newline_is_refused() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");

    open_form_mut(&mut session).push_char('\n');
    open_form_mut(&mut session).push_char('\r');
    session.type_char('\n', now);

    assert_eq!(open_form(&session).title(), "GitHub");
    assert!(!open_form(&session).is_dirty());
}

#[test]
fn zz_probe_backspace_on_empty_notes_drops_the_undo() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "Jenkins", now);
    session.begin_edit(now).expect("begin edit");
    let form = open_form_mut(&mut session);
    form.focus_next();
    form.focus_next();
    form.focus_next();
    assert_eq!(form.focus(), EditField::Notes);
    assert_eq!(form.notes(), "");
    form.backspace();
    assert!(!form.is_dirty(), "an empty-field backspace is not an edit");
    assert!(!form.undo(), "the discarded undo leaves nothing to restore");
}

#[test]
fn notes_edits_undo_and_zeroize_with_the_vault() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "Jenkins", now);
    session.begin_edit(now).expect("begin edit");
    {
        let form = open_form_mut(&mut session);
        form.focus_next();
        form.focus_next();
        form.focus_next();
        assert_eq!(form.focus(), EditField::Notes);
    }
    session.type_char('A', now);
    session.type_char('B', now);
    assert_eq!(open_form(&session).notes(), "AB");
    assert!(open_form_mut(&mut session).undo());
    assert_eq!(open_form(&session).notes(), "A");

    session.vault_mut().expect("unlocked").zeroize();
    assert_eq!(open_form(&session).notes(), "");
}
