//! Purpose: `LoopState::pump_background`: collect what finished background work changed on
//! screen, and ask for one frame when it changed anything.

use super::LoopState;
use crate::application::Redraw;
use crate::models::App;

impl LoopState {
    /// Collect what background work finished, asking for a frame when it
    /// changed what is on screen.
    pub(super) fn pump_background(&mut self, app: &mut App) {
        let pumped = [
            self.temp_cleanup.pump(app),
            self.database.pump(app),
            self.usage.pump(app),
            self.things.pump(app),
            self.remote.pump(app),
            self.git.pump(app),
            self.jobs.pump(app),
            self.tasks.pump(app),
            self.cloudflare.pump(app),
        ];
        // Every pump runs, whatever the others changed.
        let lsp = self.lsp.pump(app);
        if lsp || pumped.contains(&Redraw::Needed) {
            self.redraw = Redraw::Needed;
        }
        app.record_folder_visits();
        #[cfg(feature = "keepass")]
        if crate::ops::vault::maintenance(app) {
            self.redraw = Redraw::Needed;
        }
        self.redraw = self.redraw.or(self.images.refresh(app));
    }
}
