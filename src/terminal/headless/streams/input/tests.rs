//! Purpose: A saturated user-input queue cannot starve mandatory terminal replies.

use super::*;
use std::sync::{mpsc, Arc, Mutex};

mod delivery;
mod pump;
mod worker;

struct FirstWrite(Arc<Mutex<Vec<u8>>>);

impl Write for FirstWrite {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "fixture finished",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn terminal_reply_has_priority_over_a_full_user_queue() {
    let host = HeadlessHost::new();
    let (sender, input) = mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    host.input("private-input").unwrap();
    let (reply, replies) = mpsc::sync_channel(1);
    reply.send(Reply::CursorPosition).unwrap();
    let written = Arc::new(Mutex::new(Vec::new()));
    let result = pump(
        &mut FirstWrite(Arc::clone(&written)),
        &input,
        &replies,
        &host,
        generation,
    );
    assert!(result.is_err());
    assert_eq!(&*written.lock().unwrap(), b"\x1b[1;1R");
}
