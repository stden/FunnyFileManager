#[cfg(coverage)]
mod coverage_smoke_tests {
    use funny_file_manager::editor::handle_edit;
    use funny_file_manager::models::{App, Panel, PanelSide};
    use funny_file_manager::{coverage_touch_backend, coverage_touch_logs, WritableTestBackend};
    use ratatui::Terminal;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
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

    #[cfg(windows)]
    fn fake_editor_path(dir: &Path) -> io::Result<PathBuf> {
        let path = dir.join("fake_editor.cmd");
        fs::write(&path, "@echo off\r\nexit /b 0\r\n")?;
        Ok(path)
    }

    #[cfg(not(windows))]
    fn fake_editor_path(dir: &Path) -> io::Result<PathBuf> {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake_editor.sh");
        fs::write(&path, "#!/bin/sh\nexit 0\n")?;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
        Ok(path)
    }

    fn build_app(temp_dir: &TempDir) -> io::Result<App> {
        Ok(App {
            left_panel: Panel::new(temp_dir.path().to_path_buf())?,
            right_panel: Panel::new(temp_dir.path().to_path_buf())?,
            active_panel: PanelSide::Left,
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
            status_message: String::new(),
            quick_search: None,
        })
    }

    fn select_entry(panel: &mut Panel, name: &str) -> bool {
        if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
            panel.state.select(Some(index));
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

        let editor_path = fake_editor_path(temp_dir.path())?;
        let _guard = EditorEnvGuard::set(&editor_path);

        let mut app = build_app(&temp_dir)?;
        assert!(select_entry(&mut app.left_panel, "file.txt"));

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;

        handle_edit(&mut terminal, &mut app)?;

        assert!(app.status_message.contains("Edited"));
        Ok(())
    }
}
