//! Application coordination: draw state, read one event, and dispatch input.
//!
//! Terminal acquisition belongs to bootstrap; operation implementations belong
//! to `ops`. This layer only orders those effects and advances application state.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

use crossterm::event::{Event, KeyEventKind};
use ratatui::{backend::Backend, Terminal};

use crate::diagnostics::{key_label, log_action};
use crate::models::{App, Lifecycle, Viewport};
use crate::ui::ui;

mod dispatch;
mod input;
mod lsp_jobs;
mod timing;
mod wsl_jobs;
pub use dispatch::dispatch_key;
use dispatch::dispatch_mouse;
use input::{BufferedInput, EventSource};
use wsl_jobs::Redraw;

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

/// The next event, `None` for an idle tick, and `None` again once the session
/// has no terminal left to run in.
///
/// A lost terminal is the end of a session, not a failure of one: nothing can
/// be drawn for it and no key can ever arrive, so the loop is asked to quit
/// the way F10 asks — the caller still writes the histories and restores what
/// is left to restore. Every other read failure stays fatal, which is what
/// keeps the replay tests' end-of-script sentinel meaning what it says.
fn next_event<F>(read_event: &mut BufferedInput<F>, app: &mut App) -> io::Result<Option<Event>>
where
    F: EventSource,
{
    match read_event.read(app) {
        Ok(event) => Ok(event),
        Err(error) if crate::recording::TerminalLost::caused(&error) => {
            log_action("Controlling terminal disconnected; leaving the event loop");
            app.lifecycle = Lifecycle::Quitting;
            Ok(None)
        }
        Err(error) => Err(error),
    }
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
    app.record_folder_visits();
    crate::ops::panel_session::checkpoint(app);
    result
}

struct LoopState {
    clicks: crate::mouse::PanelClicks,
    jobs: wsl_jobs::WslJobs,
    lsp: lsp_jobs::EditorLspController,
    redraw: Redraw,
    pending: Option<timing::Pending>,
    /// The clock as the last frame showed it.
    shown_time: String,
}

impl LoopState {
    fn new(backend: Arc<dyn crate::wsl::WslBackend>) -> Self {
        Self {
            clicks: crate::mouse::PanelClicks::default(),
            jobs: wsl_jobs::WslJobs::new(backend),
            lsp: lsp_jobs::EditorLspController::default(),
            redraw: Redraw::Needed,
            pending: None,
            shown_time: String::new(),
        }
    }

    fn run<B, F>(
        &mut self,
        terminal: &mut Terminal<B>,
        app: &mut App,
        read: &mut BufferedInput<F>,
    ) -> io::Result<()>
    where
        B: Backend + Write,
        F: EventSource,
    {
        log_action("Starting event loop");
        while app.lifecycle.is_running() {
            self.prepare_frame(terminal, app)?;
            let Some(event) = next_event(read, app)? else {
                continue;
            };
            self.pending = Some(timing::Pending::start(app, &event, Instant::now()));
            self.redraw = Redraw::Needed;
            handle_event(terminal, app, event, &mut self.clicks)?;
            crate::ops::packages::run_pending(terminal, app)?;
        }
        Ok(())
    }

    fn prepare_frame<B: Backend + Write>(
        &mut self,
        terminal: &mut Terminal<B>,
        app: &mut App,
    ) -> io::Result<()> {
        self.pump_background(app);
        let time = crate::clock::now();
        if time != self.shown_time {
            // The menu bar and the viewer's status line read the clock at
            // draw time. An idle screen draws nothing, so without this it
            // would show the minute of the last key for as long as it sat.
            self.redraw = Redraw::Needed;
        }
        if self.redraw == Redraw::Unchanged {
            return Ok(());
        }
        self.shown_time = time;
        crate::ops::panel_session::checkpoint(app);
        app.expire_status_on_move();
        let rendering = Instant::now();
        let result = draw_frame(terminal, app);
        self.finish(app, rendering, Instant::now());
        self.redraw = Redraw::Unchanged;
        result
    }

    /// Collect what background work finished, asking for a frame when it
    /// changed what is on screen.
    fn pump_background(&mut self, app: &mut App) {
        if self.jobs.pump(app) == Redraw::Needed {
            self.redraw = Redraw::Needed;
        }
        if self.lsp.pump(app) {
            self.redraw = Redraw::Needed;
        }
        app.record_folder_visits();
        if crate::ops::vault::maintenance(app) {
            self.redraw = Redraw::Needed;
        }
        crate::ops::quickview::refresh(app);
    }

    fn finish(&mut self, app: &mut App, rendering: Instant, finished: Instant) {
        if let Some(pending) = self.pending.take() {
            pending.finish(&mut app.performance, rendering, finished);
        }
    }
}

fn handle_event<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    event: Event,
    panel_clicks: &mut crate::mouse::PanelClicks,
) -> io::Result<()> {
    if let Event::Paste(text) = event {
        panel_clicks.reset();
        crate::popup::paste_editor_text(app, &text);
        return Ok(());
    }
    if let Event::Resize(width, height) = event {
        return terminal
            .resize(ratatui::layout::Rect::new(0, 0, width, height))
            .map_err(|error| io::Error::other(error.to_string()));
    }
    if let Event::Mouse(mouse) = event {
        let area = terminal.get_frame().area();
        dispatch_mouse(terminal, app, mouse, area, panel_clicks);
        return Ok(());
    }
    if let Event::Key(key) = event {
        panel_clicks.reset();
        // Only process Press events, ignore Release/Repeat to avoid double navigation on Windows
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        log_action(&format!(
            "Key event: {} (modifiers: {:?})",
            key_label(key.code),
            key.modifiers
        ));

        dispatch_key(terminal, app, key);
    }
    Ok(())
}

fn draw_frame<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    // ratatui 0.30 made `Backend::Error` an associated type (io::Error for
    // crossterm, Infallible for TestBackend). Convert via Display so this stays
    // generic over B without leaking a `From` bound onto every caller.
    terminal
        .draw(|f| {
            app.set_viewport(Viewport::new(f.area().height));
            ui(f, app);
        })
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests;
