//! Purpose: Forward complete input batches to real private PTYs and capture lifecycle state.
use super::*;
use crate::terminal::headless;
use std::{
    thread,
    time::{Duration, Instant},
};

fn start(
    host: &HeadlessHost,
    script: &str,
) -> thread::JoinHandle<io::Result<headless::ExternalOutcome>> {
    let host = host.clone();
    let script = script.to_owned();
    thread::spawn(move || {
        let _scope = host.enter();
        headless::run_with_timeout(
            std::process::Command::new("sh").args(["-c", &script]),
            Some(Duration::from_secs(5)),
        )
    })
}

fn wait_for_prompt(host: &HeadlessHost) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !host.snapshot().output.contains("READY") && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        host.snapshot().output.contains("READY"),
        "{:?}",
        host.snapshot().state
    );
}

fn mixed_input() -> Vec<Event> {
    vec![
        Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        Event::Paste("текст".into()),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::SHIFT,
        }),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    ]
}

#[test]
fn key_paste_and_mouse_reach_child_in_order() {
    let host = HeadlessHost::new();
    host.resize(91, 27).unwrap();
    let child = start(
        &host,
        "stty -echo; printf READY; IFS= read -r line; printf '<%s>' \"$line\"; exit 7",
    );
    wait_for_prompt(&host);
    let state = snapshot(&host);
    assert_eq!(state["state"], "running");
    assert_eq!(state["width"], 91);
    assert_eq!(state["height"], 27);
    input(&host, mixed_input()).unwrap();
    assert_eq!(child.join().unwrap().unwrap().code, 7);
    let state = snapshot(&host);
    assert_eq!(state["state"], "exited");
    assert_eq!(state["code"], 7);
    assert!(state["output"]
        .as_str()
        .unwrap()
        .contains("<aтекст\x1b[<6;4;5M>"));
}

#[test]
fn cancelled_child_is_reported_without_an_exit_code() {
    let host = HeadlessHost::new();
    // Closing portable-pty's input writer sends a newline and EOF, which can
    // finish a shell read before cancellation reaches the process waiter.
    let child = start(&host, "printf READY; exec sleep 30");
    wait_for_prompt(&host);
    host.cancel();
    assert_eq!(
        child.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    let state = snapshot(&host);
    assert_eq!(state["state"], "cancelled");
    assert!(state["code"].is_null());
    assert!(state["output"].as_str().unwrap().contains("READY"));
}
