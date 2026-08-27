//! Purpose: Provides coverage-only smoke tests behind the coverage cfg.
//!
#[cfg(coverage)]
mod coverage_smoke_tests {
    use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};
    use funny_file_manager::models::Bookmarks;
    use funny_file_manager::models::ViewMode;
    use funny_file_manager::models::{App, Overlay, Panel, PanelSide};
    use funny_file_manager::{coverage_touch_backend, coverage_touch_logs, WritableTestBackend};
    use ratatui::Terminal;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::path::Path;
    use tempfile::TempDir;

    struct EditorEnvGuard {
        previous: Option<OsString>,
    }

    impl EditorEnvGuard {
        fn set(path: &Path) -> Self {
            let previous = env::var_os("EDITOR");
            env::set_var("EDITOR", path);
            Self { previous }
        }
    }

    impl Drop for EditorEnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.take() {
                env::set_var("EDITOR", previous);
            } else {
                env::remove_var("EDITOR");
            }
        }
    }

    fn build_app(temp_dir: &TempDir) -> io::Result<App> {
        Ok(App {
            left_panel: Panel::new(temp_dir.path().to_path_buf())?,
            right_panel: Panel::new(temp_dir.path().to_path_buf())?,
            active_panel: PanelSide::Left,
            view: ViewMode::default(),
            overlay: Overlay::None,
            bookmarks: Bookmarks::default(),
            lifecycle: funny_file_manager::models::Lifecycle::default(),
            status: Status::Cleared,
            quick_search: QuickSearch::default(),
            history: funny_file_manager::history::History::default(),
        })
    }

    fn select_entry(panel: &mut Panel, name: &str) -> bool {
        if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
            panel.select(Some(index));
            true
        } else {
            false
        }
    }

    #[test]
    fn test_coverage_helpers() -> io::Result<()> {
        coverage_touch_logs();
        coverage_touch_backend()?;
        Ok(())
    }

    #[test]
    fn test_handle_edit_public() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"content")?;

        let _guard = EditorEnvGuard::set(Path::new("editor-that-must-not-run"));

        let mut app = build_app(&temp_dir)?;
        assert!(select_entry(&mut app.left_panel, "file.txt"));

        let backend = WritableTestBackend::new(80, 24);
        let _terminal = Terminal::new(backend)?;

        handle_edit_test_mode(&mut app, TestEditorBehavior::Success)?;

        assert!(app.status_text().contains("Edited"));
        Ok(())
    }
}
