//! Purpose: The task list open over fixture processes, for the ops, popup
//! and mouse tests: panels on a temporary directory, a [`FixtureHost`]
//! installed, and one sample of the processes applied, so each test states
//! only the processes and the keys it is about.

use std::sync::Arc;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::processes::{one_core, process, FixtureHost};
use crate::models::{
    App, Niceness, Overlay, Panel, Pid, ProcessKey, ProcessSample, Sample, TaskList,
};
use crate::popup::process_popup_input;
use crate::prompt::Privilege;

/// FFM's own PID in these fixtures.
pub const OWN: u32 = 900;

pub fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a fixture PID is in range")
}

/// Process `raw` of bob's, `/usr/bin/<name>`, started at tick `raw`.
pub fn named(raw: u32, name: &str) -> ProcessSample {
    process(pid(raw), name)
}

/// The key [`named`] gives process `raw`.
pub fn key_of(raw: u32) -> ProcessKey {
    named(raw, "").key
}

pub fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a fixture nice value")
}

/// An app with the task list open and a fixture host that records what it
/// is asked instead of doing it.
pub struct TaskApp {
    pub app: App,
    pub host: FixtureHost,
    _root: TempDir,
}

impl TaskApp {
    /// The task list of the slot's first session, run by [`OWN`] with `privilege`, showing
    /// one sample of `processes`.
    pub fn open(processes: Vec<ProcessSample>, privilege: Privilege) -> Self {
        let root = TempDir::new().expect("a temporary directory");
        let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
        let mut app = App::with_panels(panel(), panel());
        let host = FixtureHost::new(vec![one_core(processes.clone())]);
        app.task_host.install(Arc::new(host.clone()));
        let session = app.task_host.next_session();
        let list = TaskList::new(session, pid(OWN), privilege);
        app.set_overlay(Overlay::Tasks(Box::new(list)));
        let mut opened = Self {
            app,
            host,
            _root: root,
        };
        opened.resample(processes);
        opened
    }

    /// The open task list.
    pub fn list(&mut self) -> &mut TaskList {
        self.app.task_list_mut().expect("the task list is open")
    }

    /// The list asks for and applies a sample of `processes`, as a refresh
    /// behind whatever is open does.
    pub fn resample(&mut self, processes: Vec<ProcessSample>) {
        self.apply(one_core(processes));
    }

    /// The list asks for and applies `sample`, as a refresh does.
    pub fn apply(&mut self, sample: Sample) {
        let list = self.list();
        let now = Instant::now();
        list.take_request(now);
        list.apply_sample(Ok(sample), now);
    }

    /// Presses `code` with `modifiers`, as the loop hands a key to the
    /// list; true when the list consumed it.
    pub fn chord(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        process_popup_input(&mut self.app, KeyEvent::new(code, modifiers))
    }

    /// Presses `code` with no modifier.
    pub fn press(&mut self, code: KeyCode) -> bool {
        self.chord(code, KeyModifiers::NONE)
    }

    /// Types `text` key by key.
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.press(KeyCode::Char(ch));
        }
    }

    /// The PIDs of the shown rows, in order.
    pub fn shown(&mut self) -> Vec<u32> {
        let list = self.list();
        list.view()
            .iter()
            .map(|row| row.key().pid().get())
            .collect()
    }
}
