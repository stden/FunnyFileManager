//! Purpose: Drain queued user input and terminate cleanly when its producer closes.

use super::super::*;
use std::sync::mpsc;

#[test]
fn pump_drains_user_input_then_exits_on_disconnection() {
    let host = HeadlessHost::new();
    let (owned_sender, _owned_input) = mpsc::sync_channel(1);
    let generation = host.begin(owned_sender).unwrap();
    let (sender, input) = mpsc::sync_channel(1);
    sender.send(b"user-text".to_vec()).unwrap();
    drop(sender);
    let (reply, replies) = mpsc::sync_channel(1);
    reply.send(Reply::CursorPosition).unwrap();
    let mut written = Vec::new();
    pump(&mut written, &input, &replies, &host, generation).unwrap();
    assert_eq!(written, b"\x1b[1;1Ruser-text");
}

#[test]
fn startup_waits_for_cursor_reply_before_draining_user_input() {
    let host = HeadlessHost::new();
    let (owned_sender, _owned_input) = mpsc::sync_channel(1);
    let generation = host.begin(owned_sender).unwrap();
    let (sender, input) = mpsc::sync_channel(1);
    sender.send(b"after-cursor".to_vec()).unwrap();
    let (reply, replies) = mpsc::sync_channel(1);
    let notifier = thread::spawn(move || {
        thread::sleep(Duration::from_millis(25));
        reply.send(Reply::CursorPosition).unwrap();
        drop(sender);
    });
    let mut written = Vec::new();
    pump_with_startup(&mut written, &input, &replies, &host, generation, true).unwrap();
    notifier.join().unwrap();
    assert_eq!(written, b"\x1b[1;1Rafter-cursor");
}
