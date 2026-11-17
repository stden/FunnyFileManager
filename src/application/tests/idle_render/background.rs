//! Purpose: a background job finishing on an idle screen repaints it once, without a key.

use super::*;

struct GatedWsl(Mutex<mpsc::Receiver<()>>);

impl WslBackend for GatedWsl {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        assert_eq!(*operation, WslOperation::Update);
        self.0
            .lock()
            .expect("completion gate")
            .recv_timeout(WAIT_LIMIT)
            .map_err(|error| error.to_string())?;
        Ok(WslOutput::Updated("Background update finished".to_owned()))
    }
}

#[test]
fn a_background_completion_repaints_once_without_a_key() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut app = app_at(directory.path())?;
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(Vec::new()))));
    crate::ops::wsl::update(&mut app);
    app.close_overlay();
    let (release, gate) = mpsc::channel();
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut polls = CompletionPolls::new(release);

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || polls.read(&screen),
        Arc::new(GatedWsl(Mutex::new(gate))),
    )?;

    assert_eq!(screen.draws(), 2);
    assert_eq!(polls.idle_after, 4);
    Ok(())
}

struct CompletionPolls {
    idle_before: usize,
    idle_after: usize,
    deadline: Instant,
    release: mpsc::Sender<()>,
}

impl CompletionPolls {
    fn new(release: mpsc::Sender<()>) -> Self {
        Self {
            idle_before: 0,
            idle_after: 0,
            deadline: Instant::now() + WAIT_LIMIT,
            release,
        }
    }
    fn read(&mut self, screen: &CountingBackend) -> io::Result<Option<Event>> {
        if self.idle_before < 4 {
            assert_eq!(screen.draws(), 1, "pending work must not redraw");
            self.idle_before += 1;
            if self.idle_before == 4 {
                self.release.send(()).expect("release background operation");
            }
            return Ok(None);
        }
        if screen.draws() == 1 {
            assert!(Instant::now() < self.deadline, "completion did not repaint");
            thread::yield_now();
            return Ok(None);
        }
        assert_eq!(screen.draws(), 2, "completed work must not keep redrawing");
        assert!(screen.text().contains("Background update finished"));
        self.idle_after += 1;
        Ok((self.idle_after == 4).then(quit))
    }
}
