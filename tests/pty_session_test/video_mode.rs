//! Purpose: Alt+F9 negotiates terminal height and reports the host's answer.

use super::*;

const ALT_F9: &[u8] = b"\x1b[20;3~";

#[test]
fn an_ignored_resize_is_reported_after_the_terminal_settles() {
    let mut ffm = Ffm::start();
    ffm.screen.clear();
    ffm.type_keys(ALT_F9);
    assert!(
        ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"\x1b[8;50;100t"))
            .is_some(),
        "Alt+F9 must request 50 rows from a 30-row terminal"
    );
    assert!(
        ffm.wait_for(PATIENCE, |ffm| {
            contains(&ffm.screen, b"Video mode: asked for 50 rows")
        })
        .is_some(),
        "an ignored request must explain the terminal's actual height: {}",
        String::from_utf8_lossy(&ffm.screen)
    );
    assert!(ffm.quit().expect("ffm exits").success());
}

#[test]
fn a_terminal_that_accepts_the_request_reports_its_new_height() {
    let mut ffm = Ffm::start();
    resize_and_answer(&mut ffm, 40);
    ffm.screen.clear();
    ffm.type_keys(ALT_F9);
    assert!(
        ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"\x1b[8;25;100t"))
            .is_some(),
        "Alt+F9 must request 25 rows from a tall terminal"
    );
    resize_and_answer(&mut ffm, 25);
    assert!(
        ffm.wait_for(PATIENCE, |ffm| contains(
            &ffm.screen,
            b"Video mode: 25 rows"
        ))
        .is_some(),
        "the status must confirm that the terminal accepted the resize: {}",
        String::from_utf8_lossy(&ffm.screen)
    );
    assert!(ffm.quit().expect("ffm exits").success());
}

fn resize_and_answer(ffm: &mut Ffm, rows: u16) {
    let boundary = ffm.screen.len();
    ffm.resize(100, rows);
    ffm.signal(Signal::WINCH);
    answer_cursor_query(ffm, boundary);
}
