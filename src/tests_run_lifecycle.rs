//! Unit tests: the process lifecycle around the event loop — the opt-in
//! logging gate, the `run_with` wrapper, and the two ways the loop can end
//! (an already-quitting app, or a terminal draw that fails).
#![allow(unused_imports)]

use super::tests::buffer_text;
use super::*;
use crate::models::Lifecycle;

use crate::models::Panel;

use ratatui::backend::{ClearType, WindowSize};
use ratatui::layout::{Position, Size};

use std::fs;

use tempfile::TempDir;

/// Message carried by [`FailingDrawBackend`]'s error, so the assertion can
/// prove the loop surfaced *that* failure rather than some other one.
const DRAW_FAILURE: &str = "backend refused to draw";

/// A backend whose `draw` always fails. Everything else delegates to the real
/// test backend, so `Terminal::new` and the resize probe still work and the
/// only failure the loop can see is the one under test.
struct FailingDrawBackend(WritableTestBackend);

impl Write for FailingDrawBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl Backend for FailingDrawBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, _content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, DRAW_FAILURE))
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.0.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.0.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.0.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.0.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.0.size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.window_size()
    }
}

/// `run_with` only wraps the runner in log lines; a successful run must stay
/// successful and the runner must be entered exactly once.
#[test]
fn run_with_runs_the_runner_once_and_keeps_its_success() {
    let mut calls = 0_u32;

    run_with(|| {
        calls += 1;
        Ok(())
    })
    .expect("a successful runner must not be turned into an error");

    assert_eq!(calls, 1, "the runner must be invoked exactly once");
}

/// The wrapper logs the failure but must not swallow or rewrite it: `main`
/// prints this error, so its text is what the user sees.
#[test]
fn run_with_propagates_the_runner_error_verbatim() {
    let error = run_with(|| Err(io::Error::other("main exploded").into()))
        .expect_err("a failing runner must not be reported as success");

    assert_eq!(error.to_string(), "main exploded");
}

/// Logging is opt-in. With `FFM_LOG` unset nothing may reach disk — that gate
/// is the reason a password typed into the editor never lands in a log file.
#[test]
fn log_error_writes_nothing_while_logging_is_off() {
    // `log_dir` memoizes its answer in a `OnceLock`, so `FFM_LOG` can only be
    // observed in the state the whole test binary shares. Take the env lock so
    // the check cannot straddle another test's env swap.
    let _lock = crate::test_support::editor_env_lock();
    if std::env::var_os("FFM_LOG").is_some() {
        return; // this run deliberately opted into logging
    }

    let log = std::env::temp_dir().join("ffm_errors.log");
    let before = fs::metadata(&log).map_or(0, |meta| meta.len());

    log_error("token=hunter2");

    let after = fs::metadata(&log).map_or(0, |meta| meta.len());
    assert_eq!(
        before,
        after,
        "an error line reached {} with FFM_LOG unset",
        log.display()
    );
}

/// Name of the child test below, as libtest spells it on the command line.
const LOG_CHILD_TEST: &str = "tests_run_lifecycle::log_lines_reach_disk_when_ffm_log_is_on";

/// The opt-in half of the logging gate, which decides *where* an enabled log
/// lands and under what name.
///
/// `log_dir` memoises its answer in a process-wide `OnceLock` and every test
/// that drives the event loop reads it first, so `FFM_LOG=1` can only be
/// observed in a fresh process. Re-run this binary for the single ignored test
/// below with the temp directory pointed at one of our own, then read back
/// exactly what it wrote.
#[test]
fn ffm_log_opt_in_writes_both_logs_into_the_temp_directory() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let child = std::process::Command::new(std::env::current_exe()?)
        .args(["--ignored", "--exact", LOG_CHILD_TEST])
        .env("FFM_LOG", "1")
        .env("TMPDIR", temp_dir.path())
        .env("TMP", temp_dir.path())
        .env("TEMP", temp_dir.path())
        .output()?;
    assert!(
        child.status.success(),
        "the child run failed:\n{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );

    let actions = temp_dir.path().join("ffm_actions.log");
    let errors = temp_dir.path().join("ffm_errors.log");
    assert!(
        fs::read_to_string(&actions)?.contains("] opened the panel"),
        "the action never reached {}",
        actions.display()
    );
    // The two logs are separate files: an error in the action log (or the other
    // way round) means the pair was crossed.
    let error_lines = fs::read_to_string(&errors)?;
    assert!(
        error_lines.contains("] ERROR: disk on fire"),
        "the error never reached {}: {error_lines}",
        errors.display()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for log in [&actions, &errors] {
            let mode = fs::metadata(log)?.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{} is readable by other users", log.display());
        }
    }
    Ok(())
}

/// Child of [`ffm_log_opt_in_writes_both_logs_into_the_temp_directory`]: it
/// only writes the two lines that test reads back. Ignored, because it is
/// meaningless in a process that did not opt in.
#[test]
#[ignore = "re-run with FFM_LOG=1 by ffm_log_opt_in_writes_both_logs_into_the_temp_directory"]
fn log_lines_reach_disk_when_ffm_log_is_on() {
    let expected = std::env::temp_dir();
    assert_eq!(
        log_dir(),
        Some(expected.as_path()),
        "FFM_LOG=1 did not reach this process"
    );

    log_action("opened the panel");
    log_error("disk on fire");
}

/// `run_app` binds the loop to crossterm's real `event::read`, which would
/// block forever. A backend that cannot draw stops it on the first frame — the
/// error can only come from inside the loop, so it proves `run_app` really
/// entered it, without ever reaching the blocking read.
#[test]
fn run_app_enters_the_loop_and_surfaces_a_failed_draw() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(FailingDrawBackend(WritableTestBackend::new(40, 8)))?;

    let error = run_app(&mut terminal, app).expect_err("a failed draw must be fatal");

    assert!(
        error.to_string().contains(DRAW_FAILURE),
        "the backend's message was lost: {error}"
    );
    Ok(())
}

/// An app that already wants to quit must leave the loop before the first
/// draw: nothing is painted and no event is consumed.
#[test]
fn event_loop_exits_before_drawing_when_the_app_already_quit() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;
    app.lifecycle = Lifecycle::Quitting;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    run_app_with_event_reader(&mut terminal, app, || {
        panic!("a quitting app must not read another event")
    })?;

    assert!(
        buffer_text(&terminal).trim().is_empty(),
        "the loop painted a frame it should never have entered"
    );
    Ok(())
}

/// A failed terminal draw is the one fatal error in the loop: it aborts
/// immediately, carrying the backend's message, without reading an event.
#[test]
fn event_loop_aborts_with_the_backend_message_when_the_draw_fails() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(FailingDrawBackend(WritableTestBackend::new(40, 8)))?;

    let error = run_app_with_event_reader(&mut terminal, app, || {
        panic!("a failed draw must abort before the next event is read")
    })
    .expect_err("a failed draw must be fatal");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(
        error.to_string().contains(DRAW_FAILURE),
        "the backend's message was lost: {error}"
    );
    Ok(())
}
