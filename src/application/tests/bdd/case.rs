//! Isolated application, backend, process host and queued-input observations.

use std::{collections::VecDeque, sync::Arc};

use crossterm::event::Event;
use ratatui::{layout::Size, Terminal};
use tempfile::TempDir;

use super::super::idle_render::backend::CountingBackend;
use super::queue::Queue;
use crate::{
    action::KeyChord,
    application::{frame::draw_frame, lsp_jobs::EditorLspController},
    models::{App, Panel, PanelSide},
    mouse::PanelClicks,
    test_support::processes::FixtureHost,
};

pub(super) struct Case {
    pub app: App,
    pub terminal: Terminal<CountingBackend>,
    pub screen: CountingBackend,
    pub host: FixtureHost,
    pub lsp: EditorLspController,
    pub lsp_pid: Option<String>,
    pub timing: Option<super::statistics::TimingSample>,
    pub queued: VecDeque<Event>,
    pub batch_frames: usize,
    pub batch_events: usize,
    pub failed_frame: bool,
    clicks: PanelClicks,
    // Drop the worker/controller before deleting its workspace on Windows.
    pub root: TempDir,
}

impl Case {
    pub fn new() -> Self {
        let root = TempDir::new().unwrap();
        for side in ["left", "right"] {
            std::fs::create_dir(root.path().join(side)).unwrap();
        }
        let mut app = application(root.path());
        let host = FixtureHost::new(Vec::new());
        app.task_host.install(Arc::new(host.clone()));
        let screen = CountingBackend::new();
        screen.resize(Size::new(120, 30));
        let terminal = Terminal::new(screen.clone()).unwrap();
        Self {
            root,
            app,
            terminal,
            screen,
            host,
            lsp: EditorLspController::default(),
            lsp_pid: None,
            timing: None,
            queued: VecDeque::new(),
            batch_frames: 0,
            batch_events: 0,
            failed_frame: false,
            clicks: PanelClicks::default(),
        }
    }

    pub fn paint(&mut self) {
        draw_frame(&mut self.terminal, &mut self.app).unwrap();
    }

    pub fn key(text: &str) -> Event {
        Event::Key(text.parse::<KeyChord>().unwrap().to_event())
    }

    pub fn press(&mut self, keys: &str) {
        for key in keys.split_whitespace() {
            crate::application::event::handle_event(
                &mut self.terminal,
                &mut self.app,
                Self::key(key),
                &mut self.clicks,
            )
            .unwrap();
            self.paint();
        }
    }

    /// Inserts `text` at the current cursor position by sending a paste event.
    pub fn type_text(&mut self, text: &str) {
        crate::application::event::handle_event(
            &mut self.terminal,
            &mut self.app,
            Event::Paste(text.to_owned()),
            &mut self.clicks,
        )
        .unwrap();
        self.paint();
    }

    pub fn batch(&mut self) {
        let before = self.screen.draws();
        self.batch_events = self.queued.len();
        let input = Queue {
            events: std::mem::take(&mut self.queued),
            screen: self.screen.clone(),
        };
        let result = crate::application::run_loop(
            &mut self.terminal,
            &mut self.app,
            input,
            Arc::new(crate::wsl::SystemWslBackend),
        );
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Interrupted);
        self.batch_frames = self.screen.draws() - before;
    }

    pub fn side(side: &str) -> PanelSide {
        match side {
            "left" => PanelSide::Left,
            "right" => PanelSide::Right,
            _ => panic!("unknown panel side {side}"),
        }
    }
}

fn application(root: &std::path::Path) -> App {
    let left = Panel::new(root.join("left")).unwrap();
    let right = Panel::new(root.join("right")).unwrap();
    let mut app = App::with_panels(left, right);
    app.config.lsp.0.clear();
    app
}
