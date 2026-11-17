//! Purpose: A failed input stream releases its detached delivery worker promptly.

use super::*;

struct WatchedWriter {
    writer: FirstWrite,
    finished: mpsc::Sender<()>,
}

impl Write for WatchedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writer.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Drop for WatchedWriter {
    fn drop(&mut self) {
        let _delivery = self.finished.send(());
    }
}

#[test]
fn failed_input_writer_releases_the_background_worker() {
    let host = HeadlessHost::new();
    let (sender, input) = mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    let (reply, replies) = mpsc::sync_channel(1);
    reply.send(Reply::CursorPosition).unwrap();
    let written = Arc::new(Mutex::new(Vec::new()));
    let (finished, observed) = mpsc::channel();
    let writer = WatchedWriter {
        writer: FirstWrite(Arc::clone(&written)),
        finished,
    };
    write(Box::new(writer), input, replies, host, generation).unwrap();
    observed.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(&*written.lock().unwrap(), Reply::bytes());
}
