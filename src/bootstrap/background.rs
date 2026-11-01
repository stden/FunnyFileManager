//! Purpose: Compose background sessions with explicit system or isolated host capabilities.
use crate::{
    automation::options::Profile,
    models::{App, Panel, TerminalMode},
};
use std::{io, path::Path};

mod clipboard;

pub(crate) fn app(root: &Path, profile: Profile) -> io::Result<App> {
    let panel = Panel::new(root.to_owned())?;
    let mut app = App::with_panels(panel.clone(), panel);
    if profile == Profile::System {
        system_hosts(&mut app);
    }
    app.clipboard
        .install(Box::new(clipboard::SessionClipboard::for_profile(profile)));
    app.live.terminal_mode = TerminalMode::Headless;
    Ok(app)
}

fn system_hosts(app: &mut App) {
    super::load_persisted(app);
    crate::ops::menu::settings::load_default(app);
    app.live = super::live_input(&crate::recording::Input::Live(None));
    if let Some(host) = crate::process_table::system_host() {
        app.task_host.install(host);
    }
    crate::ops::network_drives::start();
}

pub(crate) fn finish(app: &mut App, profile: Profile) {
    if profile == Profile::System {
        app.record_folder_visits();
        crate::ops::panel_session::flush(app);
        super::at_exit::persist_at_exit(app);
    }
    if app.clipboard.is_armed() {
        app.clipboard.disarm();
        if let Some(backend) = app.clipboard.backend_mut() {
            drop(crate::clipboard::clear(backend));
        }
    }
}

#[cfg(all(test, feature = "keepass"))]
mod tests;
