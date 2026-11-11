//! Purpose: Unit tests of the session harness: poisoned clipboards and a worker that panicked.

use std::panic::{self, AssertUnwindSafe};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use funny_file_manager::clipboard::{copy_secret, ClipboardBackend};

use super::worker::MemoryClipboard;
use super::{Output, Session};

fn poisoned_clipboard() -> MemoryClipboard {
    let shared = Arc::new(Mutex::new(String::new()));
    let holder = Arc::clone(&shared);
    let poisoner = thread::spawn(move || {
        let _guard = holder.lock();
        panic!("a step panics while it holds the clipboard");
    });
    assert!(poisoner.join().is_err());
    MemoryClipboard(shared)
}

#[test]
#[should_panic(expected = "clipboard: PoisonError")]
fn a_poisoned_clipboard_fails_a_copy_as_itself_not_as_the_osc52_tripwire() {
    drop(copy_secret(&mut poisoned_clipboard(), "text"));
}

#[test]
#[should_panic(expected = "clipboard: PoisonError")]
fn a_poisoned_clipboard_fails_a_paste_instead_of_reading_as_unavailable() {
    drop(poisoned_clipboard().get_native());
}

fn session_with_a_panicked_worker() -> Session {
    let worker = thread::spawn(|| panic!("the TUI worker fails"));
    while !worker.is_finished() {
        thread::yield_now();
    }
    let (input, _) = mpsc::channel();
    let (_, output) = mpsc::channel();
    Session {
        input,
        output,
        current: Output::Exited(Ok(())),
        worker: Some(worker),
        trace: Vec::new(),
        clipboard: Arc::default(),
    }
}

#[test]
#[should_panic(expected = "TUI worker panicked")]
fn stopping_a_session_whose_worker_panicked_fails_the_scenario() {
    session_with_a_panicked_worker().stop();
}

#[test]
fn session_debug_names_the_frames_it_kept() {
    let text = format!("{:?}", session_on_a_frame());
    assert!(text.contains("frames"), "{text}");
    assert!(text.contains('1'), "{text}");
}

#[test]
#[should_panic(expected = "expected a frame")]
fn reading_a_frame_after_exit_fails_the_scenario() {
    let _ = session_with_a_panicked_worker().buffer();
}

#[test]
#[should_panic(expected = "unknown panel")]
fn an_unknown_panel_side_fails_the_scenario() {
    drop(session_on_a_frame().panel_text("middle"));
}

#[test]
#[should_panic(expected = "unknown key")]
fn an_unknown_key_name_fails_the_scenario() {
    let _ = super::parse_key("Nope");
}

#[test]
#[should_panic(expected = "BDD must use its own clipboard")]
fn the_session_clipboard_refuses_the_terminal_osc52_path() {
    let mut clipboard = MemoryClipboard(Arc::default());
    drop(clipboard.emit_osc52("payload"));
}

fn session_on_a_frame() -> Session {
    let (input, _) = mpsc::channel();
    let (_, output) = mpsc::channel();
    Session {
        input,
        output,
        current: Output::Frame(ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(
            0, 0, 4, 2,
        ))),
        worker: None,
        trace: vec!["one".to_owned()],
        clipboard: Arc::default(),
    }
}

#[test]
fn a_session_dropped_while_its_step_unwinds_does_not_abort_the_run() {
    let session = session_with_a_panicked_worker();
    let step = panic::catch_unwind(AssertUnwindSafe(move || {
        let _session = session;
        panic!("the step fails");
    }));
    assert!(step.is_err(), "the step's own panic reaches the runner");
}
