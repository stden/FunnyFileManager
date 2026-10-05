//! Purpose: The scenario world's accessors: fixture paths, the app, the screen and key workflows.

use super::*;

impl FfmWorld {
    pub fn temp_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .temp_dir
            .as_ref()
            .context("no temp dir")?
            .path()
            .to_path_buf())
    }
    pub fn src_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .src_dir
            .as_ref()
            .context("no src dir")?
            .path()
            .to_path_buf())
    }
    pub fn dst_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .dst_dir
            .as_ref()
            .context("no dst dir")?
            .path()
            .to_path_buf())
    }
    pub fn app(&mut self) -> StepResult<&mut App> {
        self.app.as_mut().context("no app")
    }

    /// The last path segment of the source or destination directory: what a
    /// panel title shows, and unique because the directories are temporary.
    pub fn directory_name(&self, which: &str) -> StepResult<String> {
        let path = match which {
            "source" => self.src_path()?,
            _ => self.dst_path()?,
        };
        Ok(path
            .file_name()
            .context("temp dir name")?
            .to_string_lossy()
            .into_owned())
    }

    /// One 80x24 frame of the program as the user would see it.
    pub fn screen_text(&mut self) -> StepResult<String> {
        let mut terminal =
            Terminal::new(WritableTestBackend::new(80, 24)).context("test terminal")?;
        let app = self.app()?;
        terminal.draw(|frame| ui(frame, app)).context("draw")?;
        let buffer = terminal.backend().buffer();
        let rows = buffer.area.rows().map(|row| {
            row.columns()
                .map(|cell| buffer[(cell.x, cell.y)].symbol())
                .collect::<String>()
        });
        Ok(rows.collect::<Vec<_>>().join("\n"))
    }
    pub fn panel(&mut self) -> StepResult<&mut Panel> {
        self.panel.as_mut().context("no panel")
    }

    pub fn send_popup_key(&mut self, code: KeyCode) -> StepResult {
        let consumed = process_popup_input(self.app()?, KeyEvent::new(code, KeyModifiers::empty()));
        assert!(consumed, "the open dialog should consume {code:?}");
        Ok(())
    }

    pub fn type_in_popup(&mut self, text: &str) -> StepResult {
        for ch in text.chars() {
            self.send_popup_key(KeyCode::Char(ch))?;
        }
        Ok(())
    }

    pub fn run_key_workflow(&self, events: Vec<Event>, description: &str) -> StepResult {
        let app = self.app.as_ref().context("no app")?.clone();
        let mut terminal =
            Terminal::new(WritableTestBackend::new(80, 24)).context("test terminal")?;
        let mut events = events.into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "BDD event stream exhausted")
            })
        })
        .map_err(|error| format!("{description}: {error}"))
    }
}
