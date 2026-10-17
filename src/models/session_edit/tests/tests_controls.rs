//! Purpose: unit tests for the cursor walking the controls that apply, typing,
//! and the radio rows.

use super::*;

/// Tab until the cursor is on `field`; one lap of the ring at most, so a
/// broken Tab fails the test instead of hanging it.
fn tab_to(dialog: &mut SessionEdit, field: SessionField) {
    for _ in 0..SessionField::ALL.len() {
        if dialog.field() == field {
            return;
        }
        dialog.tab(true);
    }
    panic!("Tab never reached {field:?}");
}

const RING_WITHOUT_KEY_FILE: [SessionField; 9] = [
    SessionField::Name,
    SessionField::Protocol,
    SessionField::Host,
    SessionField::Port,
    SessionField::User,
    SessionField::Auth,
    SessionField::RemoteDir,
    SessionField::Ok,
    SessionField::Cancel,
];

#[test]
fn tab_walks_the_controls_in_a_ring_past_the_key_file() {
    let mut dialog = SessionEdit::create();
    for expected in RING_WITHOUT_KEY_FILE {
        assert_eq!(dialog.field(), expected);
        dialog.tab(true);
    }
    assert_eq!(dialog.field(), SessionField::Name, "the ring closes");
    dialog.tab(false);
    assert_eq!(dialog.field(), SessionField::Cancel);
    assert!(dialog.on_cancel());
    assert!(!SessionEdit::create().is_enabled(SessionField::KeyFile));
    assert!(SessionEdit::create().is_enabled(SessionField::Host));
}

#[test]
fn tab_stops_at_the_key_file_while_the_login_is_by_key() {
    let mut by_key = SessionEdit {
        auth: AuthKind::KeyFile,
        ..SessionEdit::create()
    };
    for _ in 0..6 {
        by_key.tab(true);
    }
    assert_eq!(by_key.field(), SessionField::KeyFile);
    by_key.tab(true);
    by_key.tab(false);
    assert_eq!(by_key.field(), SessionField::KeyFile, "and back");
    by_key.push('k');
    assert_eq!(by_key.key_file, "k");
}

#[test]
fn typing_edits_the_text_field_with_the_cursor() {
    let mut dialog = SessionEdit::create();
    dialog.push('w');
    dialog.push('b');
    dialog.backspace();
    assert_eq!(dialog.name, "w");
    for field in [SessionField::Host, SessionField::Port, SessionField::User] {
        tab_to(&mut dialog, field);
        dialog.push('9');
    }
    assert_eq!(dialog.host, "9");
    assert_eq!(dialog.port, "229");
    assert_eq!(dialog.user, "9");
    tab_to(&mut dialog, SessionField::RemoteDir);
    dialog.push('/');
    assert_eq!(dialog.remote_dir, "/");
}

#[test]
fn a_choice_or_a_button_takes_no_text() {
    let mut dialog = SessionEdit::create();
    for field in [SessionField::Protocol, SessionField::Ok] {
        tab_to(&mut dialog, field);
        dialog.push('x');
        dialog.backspace();
    }
    let mut untouched = SessionEdit::create();
    tab_to(&mut untouched, SessionField::Ok);
    assert_eq!(dialog, untouched);
}

#[test]
fn the_protocol_cycles_both_ways_and_the_port_follows_its_default() {
    let mut dialog = SessionEdit::create();
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Sftp, "the name is not a choice");
    tab_to(&mut dialog, SessionField::Protocol);
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Scp);
    assert_eq!(dialog.port, "22", "SCP shares SFTP's port");
    dialog.cycle_choice(true);
    assert_eq!(dialog.protocol, Protocol::Ftp);
    assert_eq!(dialog.port, "21");
    dialog.port = "2121".to_owned();
    dialog.cycle_choice(false);
    assert_eq!(dialog.protocol, Protocol::Scp);
    assert_eq!(dialog.port, "2121", "a port typed by hand stays");
}

#[test]
fn the_login_kind_cycles_both_ways() {
    let mut dialog = SessionEdit::create();
    tab_to(&mut dialog, SessionField::Auth);
    dialog.cycle_choice(true);
    assert_eq!(dialog.auth, AuthKind::KeyFile);
    dialog.cycle_choice(false);
    dialog.cycle_choice(false);
    assert_eq!(dialog.auth, AuthKind::Agent);
}
