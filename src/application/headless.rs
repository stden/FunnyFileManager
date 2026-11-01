//! Purpose: Drive the production dispatcher and background controllers on a memory terminal.
#![expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) items inside a pub(super) module that is itself inside the private application module; the pub(crate) visibility is used by crate::automation::snapshot and crate::automation::session"
)]
use super::{automation, event::handle_event, loop_state::LoopState, Redraw};
use crate::{
    automation::{catalog::Invocation, options::Profile},
    models::App,
    terminal::memory::MemoryBackend,
};
use crossterm::event::Event;
use ratatui::{
    layout::{Rect, Size},
    Terminal,
};
use std::{io, path::Path, sync::Arc};

pub(crate) struct Runtime {
    pub app: App,
    pub terminal: Terminal<MemoryBackend>,
    state: LoopState,
    clicks: crate::mouse::PanelClicks,
    profile: Profile,
}

impl Runtime {
    pub(crate) fn new(root: &Path, size: Size, profile: Profile) -> io::Result<Self> {
        let app = crate::bootstrap::background::app(root, profile)?;
        let terminal = Terminal::new(MemoryBackend::new(size.width, size.height))?;
        let mut runtime = Self {
            app,
            terminal,
            state: LoopState::new(Arc::new(crate::wsl::SystemWslBackend)),
            clicks: crate::mouse::PanelClicks::default(),
            profile,
        };
        runtime.pump()?;
        Ok(runtime)
    }

    pub(crate) fn pump(&mut self) -> io::Result<bool> {
        if let Some(size) = crate::terminal::headless::viewport() {
            let current = self.terminal.backend().buffer().area;
            if current.width != size.width || current.height != size.height {
                self.resize_memory(size)?;
            }
        }
        Ok(self
            .state
            .prepare_frame(&mut self.terminal, &mut self.app)?
            == Redraw::Needed)
    }

    pub(crate) fn invoke(&mut self, invocation: Invocation) -> io::Result<()> {
        automation::invoke(&mut self.terminal, &mut self.app, invocation)?;
        self.after_action()
    }

    pub(crate) fn input(&mut self, events: Vec<Event>) -> io::Result<()> {
        let mut iter = events.into_iter().peekable();
        while let Some(event) = iter.next() {
            if matches!(event, Event::Mouse(_)) {
                self.pump()?;
            }
            handle_event(&mut self.terminal, &mut self.app, event, &mut self.clicks)?;
            crate::ops::git_pull::run_pending(&mut self.app);
            crate::ops::packages::run_pending(&mut self.terminal, &mut self.app)?;
            if crate::ops::mcp_install::is_pending(&self.app)
                || iter
                    .peek()
                    .is_none_or(|next| matches!(next, Event::Mouse(_)))
            {
                super::loop_state::run_after_frame(&mut self.terminal, &mut self.app)?;
                self.state.redraw = Redraw::Needed;
                self.pump()?;
            }
        }
        Ok(())
    }

    pub(crate) fn resize(&mut self, size: Size) -> io::Result<()> {
        crate::terminal::headless::resize(size.width, size.height)?;
        self.resize_memory(size)?;
        self.pump().map(|_| ())
    }

    fn resize_memory(&mut self, size: Size) -> io::Result<()> {
        self.terminal.backend_mut().resize(size.width, size.height);
        self.terminal
            .resize(Rect::new(0, 0, size.width, size.height))?;
        self.state.redraw = Redraw::Needed;
        Ok(())
    }

    fn after_action(&mut self) -> io::Result<()> {
        crate::ops::git_pull::run_pending(&mut self.app);
        crate::ops::packages::run_pending(&mut self.terminal, &mut self.app)?;
        super::loop_state::run_after_frame(&mut self.terminal, &mut self.app)?;
        self.state.redraw = Redraw::Needed;
        self.pump().map(|_| ())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        crate::bootstrap::background::finish(&mut self.app, self.profile);
    }
}
