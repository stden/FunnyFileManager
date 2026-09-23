//! Purpose: Backpressure retains ownership of unsent commands and never blocks the UI.

use super::*;

#[test]
fn full_and_disconnected_mailboxes_return_the_unsent_command() {
    let (commands, receiver) = mpsc::sync_channel(1);
    let manager = Manager {
        commands,
        events: None,
        stop: Arc::new(AtomicBool::new(false)),
        task: None,
    };
    assert!(manager.send(Command::Close).is_ok());
    assert!(matches!(
        *manager.send(Command::Cancel).err().unwrap(),
        Command::Cancel
    ));
    assert!(matches!(receiver.try_recv().unwrap(), Command::Close));
    drop(receiver);
    assert!(matches!(
        *manager.send(Command::Close).err().unwrap(),
        Command::Close
    ));
    assert!(manager.poll().is_none());
}
