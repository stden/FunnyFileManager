//! Production composition: load persisted history and settings, acquire the
//! host terminal, run the application inside its terminal session, and write
//! back what the session changed.

use std::error::Error;
use std::io;
use std::path::PathBuf;

use ratatui::{backend::CrosstermBackend, Terminal};

use crate::models::App;
use crate::recording::{Input, RecordingDisabled};
use at_exit::end_session;

pub fn run() -> Result<(), Box<dyn Error>> {
    // The file manager keeps a log unless FFM_LOG=0 (see `diagnostics`).
    crate::diagnostics::log_by_default();
    crate::diagnostics::run_with(main_impl)
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(options) = crate::automation::arguments(&args) {
        return crate::automation::run(options).map_err(Into::into);
    }
    // Started by "Retry as administrator": no terminal, only the pipe.
    if let Some(served) = crate::ops::helper_main(&args) {
        return served.map_err(Into::into);
    }
    let (mut input, disabled) = Input::from_args(&args, crossterm::terminal::size()?)?;
    // Long copies, moves and deletes draw their progress on this terminal.
    let _progress = crate::progress::install(Box::new(crate::terminal::ProgressScreen::stdout()));
    run_session(&mut input, disabled)
}

fn run_session(
    input: &mut Input,
    disabled: Option<RecordingDisabled>,
) -> Result<(), Box<dyn Error>> {
    let mut app = load_app()?;
    crate::diagnostics::log_action("App initialized successfully");

    load_persisted(&mut app);
    crate::ops::menu::settings::load_default(&mut app);
    if let Some(disabled) = disabled {
        app.set_status(crate::status::Status::Error(disabled.to_string()));
    }

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

    // Only a live session has a terminal to lose; a replay ends on its own.
    #[cfg(unix)]
    if matches!(input, Input::Live(_)) {
        crate::terminal::watch_for_hang_up()?;
    }
    let mut terminal_ops = crate::terminal::CrosstermOps;
    let mut session = crate::terminal::TerminalSession::enter(&mut terminal, &mut terminal_ops)?;

    install_hosts(&mut app, input);

    let run_result = crate::application::run_loop(
        session.terminal_mut(),
        &mut app,
        input,
        std::sync::Arc::new(crate::wsl::SystemWslBackend),
    );
    let cleanup_result = session.finish();
    end_session(&app, run_result, cleanup_result)
}

/// The machine the session works on, as `input` allows: a replay gets the
/// recorded clipboard, the process table read-only, and none of what only
/// live input gets.
fn install_hosts(app: &mut App, input: &Input) {
    if matches!(input, Input::Live(_)) {
        // The terminal's image setting is not visible to child processes.
        let setting = std::env::var("FFM_IMAGE_PROTOCOL").unwrap_or_default();
        crate::ui::image::set_mode(crate::image_render::Mode::from_setting(&setting));
        crate::ops::network_drives::start();
    }
    app.clipboard
        .install(input.clipboard(Box::new(crate::clipboard::SystemClipboard::with_stdout())));
    // The task list reads and controls processes through this host. A replay
    // gets it read-only: a recorded F8 Enter must not end whatever process
    // sits under the cursor on the replaying machine.
    if let Some(host) = crate::process_table::system_host() {
        app.task_host.install(input.process_host(host));
    }
    app.live = live_input(input);
}

/// What only live input gets. The pause reads a live key; a replay has none to
/// give it. Nor does a replay get the real machine: replaying a session never
/// edits an MCP client's config, and its Language servers dialog probes
/// nothing.
fn live_input(input: &Input) -> crate::models::LiveInput {
    if !matches!(input, Input::Live(_)) {
        return crate::models::LiveInput::default();
    }
    crate::models::LiveInput {
        terminal_mode: crate::models::TerminalMode::Interactive,
        temp_cleanup: crate::models::temp_cleanup::TempSource::System,
        usage: crate::models::token_usage::UsageAccess::Network,
        things: crate::models::things::ThingsAccess::Network,
        program_pause: crate::models::ProgramPause::UntilKey,
        mcp_host: crate::ops::mcp_install::system_host(),
        lsp_probe: crate::lsp::Probe::from_process(),
        elevation: true,
    }
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
        crate::config::user_home(),
    )?;
    crate::ops::panel_session::load(&start, persistence)
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

mod at_exit;
#[allow(
    clippy::redundant_pub_crate,
    reason = "background module added during active automation development"
)]
pub(crate) mod background;

#[cfg(test)]
mod tests;
