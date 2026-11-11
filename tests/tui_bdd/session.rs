//! A persistent production event loop; every input waits for the resulting frame.

use std::sync::{Arc, Mutex};
use std::{fs, io, path::PathBuf, sync::mpsc, thread, time::Duration};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    models::{App, Panel},
    run_app_with_event_reader,
};
use ratatui::{buffer::Buffer, layout::Size, Terminal};

use crate::editor_backend::SharedBackend;

#[path = "session/keys.rs"]
mod keys;
#[path = "session/screen.rs"]
mod screen;
#[path = "session/start.rs"]
mod start;
#[path = "session/worker.rs"]
mod worker;
use keys::parse_key;
use worker::run;

const DEADLINE: Duration = Duration::from_secs(10);

enum Input {
    Event(Event),
    Stop,
}

enum Output {
    Frame(Buffer),
    Exited(io::Result<()>),
}

pub(super) struct Session {
    input: mpsc::Sender<Input>,
    output: mpsc::Receiver<Output>,
    current: Output,
    worker: Option<thread::JoinHandle<()>>,
    trace: Vec<String>,
    clipboard: Arc<Mutex<String>>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("frames", &self.trace.len())
            .finish_non_exhaustive()
    }
}

impl Session {
    #[expect(
        clippy::expect_used,
        reason = "the BDD harness: a TUI that stops answering or panics fails the scenario driving it"
    )]
    fn receive(&mut self, label: &str) {
        self.current = self
            .output
            .recv_timeout(DEADLINE)
            .expect("TUI response within 10 seconds");
        let text = match &self.current {
            Output::Frame(buffer) => buffer_text(buffer, 0..buffer.area.width),
            Output::Exited(result) => format!("Exited: {result:?}"),
        };
        self.trace.push(format!("{label}\n{text}\n"));
    }

    #[expect(
        clippy::expect_used,
        reason = "the BDD harness: a TUI that stops answering or panics fails the scenario driving it"
    )]
    fn event(&mut self, event: Event) {
        assert!(
            matches!(self.current, Output::Frame(_)),
            "TUI already exited"
        );
        let label = format!("{event:?}");
        self.input.send(Input::Event(event)).expect("send input");
        self.receive(&label);
    }

    pub(super) fn press(&mut self, key: &str) {
        self.event(Event::Key(parse_key(key)));
    }

    pub(super) fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.event(Event::Key(KeyEvent::new(
                KeyCode::Char(ch),
                KeyModifiers::NONE,
            )));
        }
    }

    pub(super) fn resize(&mut self, size: Size) {
        self.event(Event::Resize(size.width, size.height));
    }

    pub(super) fn paste(&mut self, text: &str) {
        self.event(Event::Paste(text.to_owned()));
    }

    #[expect(
        clippy::expect_used,
        reason = "the BDD harness: a TUI worker that panicked fails the scenario that stops it"
    )]
    pub(super) fn stop(&mut self) {
        if matches!(self.current, Output::Frame(_)) && self.input.send(Input::Stop).is_ok() {
            if let Ok(output) = self.output.recv_timeout(DEADLINE) {
                self.current = output;
            }
        }
        if let Some(worker) = self.worker.take().filter(thread::JoinHandle::is_finished) {
            let joined = worker.join();
            // Drop runs this too. While a failed step unwinds, a second panic
            // would abort the whole run, and that step already fails the scenario.
            if !thread::panicking() {
                joined.expect("TUI worker panicked");
            }
        }
    }

    pub(super) fn save_trace(&self, name: &str) -> io::Result<()> {
        let directory = std::env::var_os("FFM_TUI_ARTIFACT_DIR")
            .map_or_else(|| PathBuf::from("target/tui-bdd"), PathBuf::from);
        fs::create_dir_all(&directory)?;
        let name: String = name
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        fs::write(directory.join(format!("{name}.txt")), self.trace.join("\n"))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

fn buffer_text(buffer: &Buffer, columns: std::ops::Range<u16>) -> String {
    (0..buffer.area.height)
        .map(|y| {
            columns
                .clone()
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "session/tests.rs"]
mod tests;
