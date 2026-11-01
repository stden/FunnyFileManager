//! Purpose: Execute named automation commands through the application's canonical interpreters.

use std::io::{self, Write};

use ratatui::{backend::Backend, Terminal};

use crate::{
    automation::catalog::Invocation,
    models::{App, PanelKind},
};

mod context;
mod inject;

#[expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) re-export inside a pub(super) parent: visibility is intentionally crate-wide for the snapshot module"
)]
pub(crate) use context::current_context;

#[expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) fn inside a pub(super) parent: visibility is intentionally crate-wide for the headless runtime"
)]
pub(crate) fn invoke<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    invocation: Invocation,
) -> io::Result<()> {
    context::validate(app, invocation)?;
    match invocation {
        Invocation::Panels(action) => super::dispatch::dispatch_action(terminal, app, action),
        Invocation::TreePanel(command) => {
            crate::popup::tree::apply(app, command);
        }
        Invocation::NetboxPanel(command) => match app.get_active_panel_ref().kind {
            PanelKind::Sessions(_) => {
                crate::popup::sessions::apply(app, command);
            }
            PanelKind::Remote(_) => {
                crate::popup::remote::apply(app, command);
            }
            _ => {}
        },
        Invocation::TaskListSearch(_) => super::dispatch_key(terminal, app, inject::find_next()),
        Invocation::TaskList(command) => {
            crate::popup::tasks::invoke(app, command);
        }
        _ => inject::dispatch(terminal, app, invocation),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
