//! Purpose: Starting a session: plain panels, saved settings, a fake machine or a resumed file.

use super::*;

impl Session {
    /// Start as [`Self::with_settings`] does, on a machine with `bin` as its
    /// whole `PATH` and `cache` as its LSP cache: nothing depends on the real
    /// home, `PATH` or LSP cache.
    pub(in crate::tui_bdd) fn language_servers(
        panels: (PathBuf, PathBuf),
        settings: PathBuf,
        machine: (PathBuf, PathBuf),
        size: Size,
    ) -> Self {
        Self::start_app(
            move || {
                let (left, right) = panels;
                let (cache, bin) = machine;
                let mut app = App::with_panels(Panel::new(left)?, Panel::new(right)?);
                funny_file_manager::test_support::restore_settings(&mut app, settings);
                funny_file_manager::test_support::set_lsp_probe(&mut app, &cache, &[bin.as_path()]);
                Ok(app)
            },
            size,
        )
    }

    pub(in crate::tui_bdd) fn start(left: PathBuf, right: PathBuf, size: Size) -> Self {
        Self::start_app(
            move || Ok(App::with_panels(Panel::new(left)?, Panel::new(right)?)),
            size,
        )
    }

    /// Start as production does, with the Options pages loaded from
    /// `settings` instead of the user's config home.
    pub(in crate::tui_bdd) fn with_settings(
        left: PathBuf,
        right: PathBuf,
        settings: PathBuf,
        size: Size,
    ) -> Self {
        Self::start_app(
            move || {
                let mut app = App::with_panels(Panel::new(left)?, Panel::new(right)?);
                funny_file_manager::test_support::restore_settings(&mut app, settings);
                Ok(app)
            },
            size,
        )
    }

    /// The same, looking at a fake machine for the MCP installer.
    pub(in crate::tui_bdd) fn start_on(
        left: PathBuf,
        right: PathBuf,
        size: Size,
        host: funny_file_manager::mcp_clients::Host,
    ) -> Self {
        Self::start_app(
            move || {
                let mut app = App::with_panels(Panel::new(left)?, Panel::new(right)?);
                app.live.mcp_host = host;
                Ok(app)
            },
            size,
        )
    }

    pub(in crate::tui_bdd) fn resume(
        directory: &std::path::Path,
        file: PathBuf,
        size: Size,
    ) -> Self {
        let directory = directory.to_owned();
        Self::start_app(
            move || {
                funny_file_manager::ops::panel_session::load(
                    &directory,
                    funny_file_manager::history::Persistence::File(file),
                )
            },
            size,
        )
    }

    fn start_app(
        create_app: impl FnOnce() -> io::Result<App> + Send + 'static,
        size: Size,
    ) -> Self {
        let (input, incoming) = mpsc::channel();
        let (outgoing, output) = mpsc::channel();
        let clipboard = Arc::new(Mutex::new(String::new()));
        let copied = Arc::clone(&clipboard);
        let worker = thread::spawn(move || {
            let result = create_app().and_then(|app| run(app, size, &incoming, &outgoing, copied));
            // A session dropped before the app exits no longer listens.
            drop(outgoing.send(Output::Exited(result)));
        });
        let mut session = Self {
            input,
            output,
            current: Output::Exited(Ok(())),
            worker: Some(worker),
            trace: Vec::new(),
            clipboard,
        };
        session.receive("Initial screen");
        session
    }
}
