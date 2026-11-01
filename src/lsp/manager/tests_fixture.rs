//! Purpose: A controlled LSP mailbox for tests of a full or disconnected worker.
use super::*;

pub fn detached(capacity: usize) -> (Manager, Receiver<Command>) {
    let (commands, receiver) = mpsc::sync_channel(capacity);
    let manager = Manager {
        commands: Some(commands),
        events: None,
        stop: Arc::new(AtomicBool::new(false)),
        task: None,
    };
    (manager, receiver)
}
