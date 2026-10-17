//! Purpose: The worker thread: the production event loop fed from a channel, with its own
//! clipboard.

use super::*;

pub fn run(
    mut app: App,
    size: Size,
    input: &mpsc::Receiver<Input>,
    output: &mpsc::Sender<Output>,
    clipboard: Arc<Mutex<String>>,
) -> io::Result<()> {
    app.clipboard.install(Box::new(MemoryClipboard(clipboard)));
    let backend = SharedBackend::new(size.width, size.height);
    let mut terminal = Terminal::new(backend.clone())?;
    run_app_with_event_reader(&mut terminal, app, || {
        output
            .send(Output::Frame(backend.buffer()))
            .map_err(io::Error::other)?;
        match input.recv_timeout(DEADLINE).map_err(io::Error::other)? {
            Input::Event(event) => {
                if let Event::Resize(width, height) = event {
                    backend.resize(Size::new(width, height));
                }
                Ok(event)
            }
            Input::Stop => Err(io::Error::new(io::ErrorKind::Interrupted, "BDD cleanup")),
        }
    })
}

pub struct MemoryClipboard(pub Arc<Mutex<String>>);

impl funny_file_manager::clipboard::ClipboardBackend for MemoryClipboard {
    #[expect(
        clippy::expect_used,
        reason = "a poisoned lock means a panic already failed the scenario; Err would show as a failed paste"
    )]
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.0.lock().expect("clipboard").clone())
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned lock means a panic already failed the scenario; Err would hit the OSC 52 tripwire"
    )]
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        text.clone_into(&mut self.0.lock().expect("clipboard"));
        Ok(())
    }
    #[expect(
        clippy::panic,
        reason = "a tripwire: BDD reaching the terminal clipboard is a test bug and fails the scenario"
    )]
    fn emit_osc52(&mut self, _: &str) -> Result<(), String> {
        panic!("BDD must use its own clipboard")
    }
}
