//! Production composition: load persisted history and settings, acquire the
//! host terminal, run the application inside its terminal session, and write
//! back what the session changed.

use std::error::Error;
use std::io;
use std::path::PathBuf;

use ratatui::{backend::CrosstermBackend, Terminal};

use crate::models::App;

pub fn run() -> Result<(), Box<dyn Error>> {
    crate::diagnostics::run_with(main_impl)
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let mut input = crate::recording::Input::from_args(&args, crossterm::terminal::size()?)?;
    run_session(&mut input)
}

fn run_session(input: &mut crate::recording::Input) -> Result<(), Box<dyn Error>> {
    let mut app = load_app()?;
    crate::diagnostics::log_action("App initialized successfully");

    load_persisted(&mut app);
    crate::ops::menu::settings::load_default(&mut app);

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(backend, input.terminal_options())?;

    // `panic = "abort"` skips Drop. The hook still gets one best-effort chance
    // to restore the host terminal before the process terminates. Install it
    // before setup mutates terminal state because setup itself may panic.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        crate::terminal::restore_after_abort();
        previous_hook(info);
    }));

    let mut terminal_ops = crate::terminal::CrosstermOps;
    let mut session = crate::terminal::TerminalSession::enter(&mut terminal, &mut terminal_ops)?;

    app.clipboard
        .install(input.clipboard(Box::new(crate::clipboard::SystemClipboard::with_stdout())));
    // The pause reads a live key; a replay has none to give it.
    if matches!(input, crate::recording::Input::Live(_)) {
        app.program_pause = crate::models::ProgramPause::UntilKey;
    }

    let run_result = crate::application::run_loop(
        session.terminal_mut(),
        &mut app,
        input,
        std::sync::Arc::new(crate::wsl::SystemWslBackend),
    );
    let cleanup_result = session.finish();
    end_session(&app, run_result, cleanup_result)
}

/// Report how the session ended, and hand `main` its outcome.
///
/// A terminal that hung up can be neither drawn on nor restored, so the errors
/// that say so are how the session ended, not a failure of it: the loop quits
/// on a hang-up the way F10 asks. Reporting them would also write to the dead
/// terminal, and `eprintln!` panics when that write fails.
fn end_session(
    app: &App,
    run_result: io::Result<()>,
    cleanup_result: io::Result<()>,
) -> Result<(), Box<dyn Error>> {
    if crate::recording::wait_for_terminal(std::time::Duration::ZERO).is_err() {
        report_at_exit(app, &Ok(()));
        return Ok(());
    }
    report_at_exit(app, &run_result);
    crate::terminal::merge_run_and_cleanup(run_result, cleanup_result).map_err(Into::into)
}

fn load_app() -> io::Result<App> {
    let persistence = crate::config::dir()
        .map_or(crate::history::Persistence::SessionOnly, |dir| {
            crate::history::Persistence::File(dir.join("panel-session.json"))
        });
    // The launch directory may have been deleted before ffm started; then
    // `getcwd` fails and the start falls back instead of aborting.
    let start = crate::ops::panel_session::start_directory(
        std::env::current_dir(),
        std::env::var_os("PWD").map(PathBuf::from),
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from),
    )?;
    crate::ops::panel_session::load(&start, persistence)
}

/// Everything that has to wait for the screen to be the user's again: the
/// state written back to disk, a recording that gave up halfway, and the
/// failure that ended the session, if one did.
#[expect(
    clippy::print_stderr,
    reason = "runs after the terminal is restored: stderr is the only channel left for a failure"
)]
fn report_at_exit(app: &App, run_result: &io::Result<()>) {
    persist_at_exit(app);
    if let Err(error) = app.performance.save_session() {
        eprintln!("ffm: could not save action statistics: {error}");
    }
    if let Some(reason) = crate::recording::stopped_reason() {
        eprintln!("ffm: recording stopped: {reason}");
    }
    if let Err(error) = run_result {
        eprintln!("{}", AtExitErrorLine(error));
    }
}

/// The line a failed session leaves on stderr: the `io::Error` in its Debug
/// form, which also names the `ErrorKind`. `main` prints the Display form
/// after it.
struct AtExitErrorLine<'error>(&'error io::Error);

impl std::fmt::Display for AtExitErrorLine<'_> {
    #[expect(
        clippy::use_debug,
        reason = "the exit line deliberately shows the io::Error's Debug form, which names its ErrorKind; main prints the Display form after it"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {:?}", self.0)
    }
}

/// Load the command history, the settings, the view and folder histories,
/// the bookmarks and the stored sessions at the production entry point.
/// `App::new()` scans the initial panels but
/// creates only session-local state, so constructing an App cannot pollute
/// the user's real files. Tests exercise the application with explicitly
/// injected state and event sources; persistence is covered in each store's
/// own tests.
fn load_persisted(app: &mut App) {
    app.history = crate::history::History::load();
    app.config = crate::config::Settings::load();
    app.adopt_description_names();
    app.view_history = crate::models::ViewHistory::load();
    app.folder_history = crate::models::FolderHistory::load();
    app.bookmarks = crate::models::Bookmarks::load();
    crate::ops::sessions::install(app, crate::sessions::SessionStore::load());
}

/// Write the histories, and the settings when "Auto save setup" is on, after
/// the terminal is back in the user's hands. Failures are logged, never
/// raised: a full disk must not turn a clean exit into an error report.
fn persist_at_exit(app: &App) {
    let mut writes = vec![
        ("view history", app.view_history.save()),
        ("folder history", app.folder_history.save()),
    ];
    if app.config.system.auto_save_setup {
        writes.push(("settings", app.config.save()));
    }
    for (what, result) in writes {
        if let Err(error) = result {
            crate::diagnostics::log_error(&format!("saving {what}: {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
