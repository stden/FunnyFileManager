//! Application coordination: draw state, read one event, and dispatch input.
//!
//! Terminal acquisition belongs to bootstrap; operation implementations belong
//! to `ops`. This layer only orders those effects and advances application state.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

use crossterm::event::Event;
use ratatui::{backend::Backend, Terminal};

use crate::models::App;

mod dispatch;
mod event;
mod frame;
mod git_jobs;
mod image_preview_jobs;
mod input;
mod loop_state;
mod lsp_jobs;
mod navigation_batch;
mod remote_jobs;
mod system_temp_cleanup_jobs;
mod task_jobs;
mod things_jobs;
mod timing;
mod token_usage_jobs;
mod wsl_jobs;
pub use dispatch::dispatch_key;
use input::{BufferedInput, EventSource};
use loop_state::LoopState;

/// Whether pumping background work produced an outcome for presentation.
///
/// A loop concept rather than one worker's: the worker controllers answer
/// with it (`WslJobs` and the task list's `TaskJobs`), and the loop draws a
/// frame only on `Needed`. The LSP controller and vault maintenance still
/// answer with a `bool`, which the loop turns into `Needed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Redraw {
    Unchanged,
    Needed,
}

impl Redraw {
    /// `Needed` when either step asked for a frame.
    const fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unchanged, Self::Unchanged) => Self::Unchanged,
            (Self::Needed, _) | (_, Self::Needed) => Self::Needed,
        }
    }
}

/// Intended clipboard retention interval for the optional vault integration.
/// Kept at the public crate path for compatibility with existing callers.
#[cfg(feature = "keepass")]
pub const CLIPBOARD_CLEAR: std::time::Duration = std::time::Duration::from_secs(30);

/// Run an initialized application, polling for keys and background completions.
/// Terminal setup and restoration remain the caller's responsibility.
pub fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    run_loop(
        terminal,
        &mut app,
        &mut crate::recording::Input::Live(None),
        Arc::new(crate::wsl::SystemWslBackend),
    )
}

/// Run the same application loop with a caller-supplied event source.
/// Drawing and event-read failures terminate the loop; operation errors update status.
pub fn run_app_with_event_reader<B, F>(
    terminal: &mut Terminal<B>,
    app: App,
    mut read_event: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnMut() -> io::Result<Event>,
{
    run_app_with_wsl_backend(
        terminal,
        app,
        || read_event().map(Some),
        Arc::new(crate::wsl::SystemWslBackend),
    )
}

/// The production loop with explicit event and WSL adapters for deterministic tests.
/// A `None` event is a timer tick, allowing maintenance to finish without a key.
pub fn run_app_with_wsl_backend<B, F>(
    terminal: &mut Terminal<B>,
    mut app: App,
    read_event: F,
    backend: Arc<dyn crate::wsl::WslBackend>,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnMut() -> io::Result<Option<Event>>,
{
    run_loop(terminal, &mut app, read_event, backend)
}

/// The loop itself, on a borrowed `App` so the production entry point still
/// holds the state when the loop returns — it has settings and histories to
/// write at exit, and only it knows where.
pub fn run_loop<B, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    read_event: F,
    backend: Arc<dyn crate::wsl::WslBackend>,
) -> io::Result<()>
where
    B: Backend + Write,
    F: EventSource,
{
    let mut state = LoopState::new(backend);
    let result = state.run(terminal, app, &mut BufferedInput::new(read_event));
    let finished = Instant::now();
    state.finish(app, finished, finished);
    if app.clipboard.is_armed() {
        app.clipboard.disarm();
        if let Some(backend) = app.clipboard.backend_mut() {
            drop(crate::clipboard::clear(backend));
        }
    }
    app.record_folder_visits();
    crate::ops::panel_session::flush(app);
    result
}

#[cfg(test)]
mod tests;
