//! Purpose: `LoopState` — one turn of the event loop: pump background work, draw a frame only when something changed, then handle the next event.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

use ratatui::{backend::Backend, layout::Size, Terminal};

use super::event::{discard_typeahead, handle_event, next_event, UNANSWERED};
pub(super) use super::frame::draw_frame;
use super::input::{BufferedInput, EventSource};
use super::{git_jobs, image_preview_jobs, lsp_jobs, task_jobs, timing, wsl_jobs, Redraw};
use crate::diagnostics::log_action;
use crate::models::App;
pub(super) struct LoopState {
    clicks: crate::mouse::PanelClicks,
    git: git_jobs::GitDirtyJobs,
    images: image_preview_jobs::ImagePreviewJobs,
    jobs: wsl_jobs::WslJobs,
    tasks: task_jobs::TaskJobs,
    lsp: lsp_jobs::EditorLspController,
    remote: super::remote_jobs::RemoteJobs,
    temp_cleanup: super::system_temp_cleanup_jobs::TempCleanupJobs,
    usage: super::token_usage_jobs::UsageJobs,
    things: super::things_jobs::ThingsJobs,
    redraw: Redraw,
    pending: Option<timing::Pending>,
    /// The clock as the last frame showed it.
    shown_time: String,
    terminal_size: Option<Size>,
}

impl LoopState {
    pub(super) fn new(backend: Arc<dyn crate::wsl::WslBackend>) -> Self {
        Self {
            clicks: crate::mouse::PanelClicks::default(),
            git: git_jobs::GitDirtyJobs::default(),
            images: image_preview_jobs::ImagePreviewJobs::default(),
            jobs: wsl_jobs::WslJobs::new(backend),
            tasks: task_jobs::TaskJobs::default(),
            lsp: lsp_jobs::EditorLspController::default(),
            remote: super::remote_jobs::RemoteJobs::default(),
            temp_cleanup: super::system_temp_cleanup_jobs::TempCleanupJobs::default(),
            usage: super::token_usage_jobs::UsageJobs::default(),
            things: super::things_jobs::ThingsJobs::default(),
            redraw: Redraw::Needed,
            pending: None,
            shown_time: String::new(),
            terminal_size: None,
        }
    }

    pub(super) fn run<B, F>(
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
            let started = Instant::now();
            let navigation = super::navigation_batch::accepts(app, &event);
            let pending = timing::Pending::start(app, &event, started);
            if handle_event(terminal, app, event, &mut self.clicks)? == Redraw::Needed {
                self.pending = Some(pending);
                self.redraw = Redraw::Needed;
            }
            if navigation {
                super::navigation_batch::drain(terminal, app, read, &mut self.clicks, started)?;
            }
            crate::ops::git_pull::run_pending(app);
            repaint_after_progress(terminal)?;
            crate::ops::packages::run_pending(terminal, app)?;
            run_after_frame(terminal, app)?;
            if started.elapsed() >= UNANSWERED {
                discard_typeahead(read, app)?;
            }
        }
        Ok(())
    }
    fn prepare_frame<B: Backend + Write>(
        &mut self,
        terminal: &mut Terminal<B>,
        app: &mut App,
    ) -> io::Result<()> {
        self.pump_background(app);
        let size = terminal
            .size()
            .map_err(|error| io::Error::other(error.to_string()))?;
        if self
            .terminal_size
            .replace(size)
            .is_some_and(|old| old != size)
        {
            terminal
                .clear()
                .map_err(|error| io::Error::other(error.to_string()))?;
            self.redraw = Redraw::Needed;
        }
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
        let pumped = [
            self.temp_cleanup.pump(app),
            self.usage.pump(app),
            self.things.pump(app),
            self.remote.pump(app),
            self.git.pump(app),
            self.jobs.pump(app),
            self.tasks.pump(app),
        ];
        // Every pump runs, whatever the others changed.
        let lsp = self.lsp.pump(app);
        if lsp || pumped.contains(&Redraw::Needed) {
            self.redraw = Redraw::Needed;
        }
        app.record_folder_visits();
        if crate::ops::vault::maintenance(app) {
            self.redraw = Redraw::Needed;
        }
        self.redraw = self.redraw.or(self.images.refresh(app));
    }

    pub(super) fn finish(&mut self, app: &mut App, rendering: Instant, finished: Instant) {
        if let Some(pending) = self.pending.take() {
            pending.finish(&mut app.performance, rendering, finished);
        }
    }
}

/// Work that has to be seen starting: Register in the Install MCP server
/// dialog may wait on `claude` for up to a minute, so the frame in which the
/// dialog says so is drawn before it runs.
pub(super) fn run_after_frame<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    if !crate::ops::mcp_install::is_pending(app) {
        return Ok(());
    }
    draw_frame(terminal, app)?;
    crate::ops::mcp_install::run_pending(app);
    Ok(())
}

/// Forget what the last frame put on the screen when a progress dialog was
/// painted over it, so the next frame redraws every cell rather than only
/// the ones it thinks changed — the dialog's among them.
pub(super) fn repaint_after_progress<B: Backend + Write>(
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    if !crate::progress::take_painted() {
        return Ok(());
    }
    // Wipe the glass and both diff buffers. `Terminal::clear` would also ask
    // the host where the cursor is and wait for the answer, and a terminal
    // that does not answer would end the session after the progress dialog.
    terminal
        .backend_mut()
        .clear()
        .map_err(|error| io::Error::other(error.to_string()))?;
    terminal.current_buffer_mut().reset();
    terminal.swap_buffers();
    Ok(())
}

#[cfg(all(test, feature = "keepass"))]
mod tests;
