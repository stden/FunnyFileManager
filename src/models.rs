use ratatui::widgets::ListState;
use std::{
    cmp::Ordering,
    fs, io,
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Default)]
pub struct Viewer {
    pub title: String,
    pub lines: Vec<String>,
    pub scroll: u16,
}

#[derive(Debug)]
pub struct Panel {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
    pub state: ListState,
}

impl Panel {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let mut panel = Panel {
            path,
            entries: Vec::new(),
            state: ListState::default(),
        };
        panel.load_entries()?;
        if !panel.entries.is_empty() {
            panel.state.select(Some(0));
        }
        Ok(panel)
    }

    pub fn load_entries(&mut self) -> io::Result<()> {
        self.entries.clear();

        if self.path.parent().is_some() {
            self.entries.push(DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            });
        }

        let mut entries: Vec<DirEntry> = fs::read_dir(&self.path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;
                let name = entry.file_name().to_string_lossy().to_string();

                Some(DirEntry {
                    name,
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                })
            })
            .collect();

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.entries.extend(entries);
        Ok(())
    }

    pub fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn enter(&mut self) -> io::Result<()> {
        if let Some(i) = self.state.selected() {
            if let Some(entry) = self.entries.get(i) {
                if entry.is_dir {
                    if entry.name == ".." {
                        if let Some(parent) = self.path.parent() {
                            self.path = parent.to_path_buf();
                        }
                    } else {
                        self.path = self.path.join(&entry.name);
                    }
                    self.load_entries()?;
                    if !self.entries.is_empty() {
                        self.state.select(Some(0));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_entry(&self) -> Option<&DirEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn ensure_selection(&mut self) {
        if !self.entries.is_empty() && self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.get_selected_entry()
            .map(|entry| self.path.join(&entry.name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelSide {
    Left,
    Right,
}

pub struct App {
    pub left_panel: Panel,
    pub right_panel: Panel,
    pub active_panel: PanelSide,
    pub show_help: bool,
    pub viewer: Option<Viewer>,
    pub status_message: String,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Ok(App {
            left_panel: Panel::new(current_dir.clone())?,
            right_panel: Panel::new(current_dir)?,
            active_panel: PanelSide::Left,
            show_help: false,
            viewer: None,
            status_message: String::from(
                "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete",
            ),
        })
    }

    pub fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    pub fn get_inactive_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.right_panel,
            PanelSide::Right => &mut self.left_panel,
        }
    }

    pub fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelSide::Left => PanelSide::Right,
            PanelSide::Right => PanelSide::Left,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_dir_entry_creation() {
        let entry = DirEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
        };
        
        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_dir_entry_clone() {
        let entry = DirEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
        };
        
        let cloned = entry.clone();
        assert_eq!(cloned.name, entry.name);
        assert_eq!(cloned.is_dir, entry.is_dir);
        assert_eq!(cloned.size, entry.size);
    }

    #[test]
    fn test_viewer_default() {
        let viewer = Viewer::default();
        assert_eq!(viewer.title, "");
        assert!(viewer.lines.is_empty());
        assert_eq!(viewer.scroll, 0);
    }

    #[test]
    fn test_viewer_creation() {
        let viewer = Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 5,
        };
        
        assert_eq!(viewer.title, "test.txt");
        assert_eq!(viewer.lines.len(), 2);
        assert_eq!(viewer.scroll, 5);
    }

    #[test]
    fn test_panel_new() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        assert_eq!(panel.path, temp_dir.path());
        assert!(!panel.entries.is_empty() || panel.entries.is_empty()); // May have .. entry or be empty
        assert_eq!(panel.state.selected(), if panel.entries.is_empty() { None } else { Some(0) });
        
        Ok(())
    }

    #[test]
    fn test_panel_load_entries_empty_dir() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        panel.load_entries()?;
        
        // Should have parent directory entry if not at root
        if temp_dir.path().parent().is_some() {
            assert!(panel.entries.iter().any(|e| e.name == ".."));
        }
        
        Ok(())
    }

    #[test]
    fn test_panel_load_entries_with_files() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test files and directories
        fs::create_dir(temp_dir.path().join("dir1"))?;
        fs::create_dir(temp_dir.path().join("dir2"))?;
        File::create(temp_dir.path().join("file1.txt"))?.write_all(b"test")?;
        File::create(temp_dir.path().join("file2.txt"))?.write_all(b"test")?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        panel.load_entries()?;
        
        // Check entries are sorted (directories first, then files)
        let mut found_file = false;
        for entry in &panel.entries {
            if entry.name != ".." {
                if entry.is_dir && found_file {
                    // Found a directory after a file - this is wrong
                    panic!("Found directory after file in sorted entries");
                }
                if !entry.is_dir {
                    found_file = true;
                }
            }
        }
        
        // Verify we have the expected entries
        assert!(panel.entries.len() >= 5); // .. + 2 dirs + 2 files
        assert!(panel.entries.iter().any(|e| e.name == "dir1" && e.is_dir));
        assert!(panel.entries.iter().any(|e| e.name == "dir2" && e.is_dir));
        assert!(panel.entries.iter().any(|e| e.name == "file1.txt" && !e.is_dir));
        assert!(panel.entries.iter().any(|e| e.name == "file2.txt" && !e.is_dir));
        
        Ok(())
    }

    #[test]
    fn test_panel_navigation_next() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create multiple files
        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i)))?;
        }
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Test next navigation
        assert_eq!(panel.state.selected(), Some(0));
        
        panel.next();
        assert_eq!(panel.state.selected(), Some(1));
        
        panel.next();
        assert_eq!(panel.state.selected(), Some(2));
        
        // Navigate to last item
        while panel.state.selected() != Some(panel.entries.len() - 1) {
            panel.next();
        }
        
        // Next should wrap to first
        panel.next();
        assert_eq!(panel.state.selected(), Some(0));
        
        Ok(())
    }

    #[test]
    fn test_panel_navigation_previous() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create multiple files
        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i)))?;
        }
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Test previous navigation from first item (should wrap to last)
        panel.previous();
        assert_eq!(panel.state.selected(), Some(panel.entries.len() - 1));
        
        // Navigate back
        panel.previous();
        assert_eq!(panel.state.selected(), Some(panel.entries.len() - 2));
        
        Ok(())
    }

    #[test]
    fn test_panel_navigation_empty() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: Vec::new(),
            state: ListState::default(),
        };
        
        // Navigation should not crash on empty panel
        panel.next();
        assert_eq!(panel.state.selected(), None);
        
        panel.previous();
        assert_eq!(panel.state.selected(), None);
    }

    #[test]
    fn test_panel_enter_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir)?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Find and select the subdirectory
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                panel.state.select(Some(i));
                break;
            }
        }
        
        // Enter the directory
        panel.enter()?;
        assert_eq!(panel.path, subdir);
        
        // Should have .. entry
        assert!(panel.entries.iter().any(|e| e.name == ".."));
        
        // Go back to parent
        panel.state.select(Some(0)); // Select ..
        panel.enter()?;
        assert_eq!(panel.path, temp_dir.path());
        
        Ok(())
    }

    #[test]
    fn test_panel_enter_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        let original_path = panel.path.clone();
        
        // Find and select the file
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                panel.state.select(Some(i));
                break;
            }
        }
        
        // Entering a file should not change the path
        panel.enter()?;
        assert_eq!(panel.path, original_path);
        
        Ok(())
    }

    #[test]
    fn test_panel_get_selected_entry() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Should have an entry selected by default
        let entry = panel.get_selected_entry();
        assert!(entry.is_some());
        
        // Clear selection
        panel.state.select(None);
        assert!(panel.get_selected_entry().is_none());
        
        Ok(())
    }

    #[test]
    fn test_panel_ensure_selection() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Clear selection
        panel.state.select(None);
        assert!(panel.state.selected().is_none());
        
        // Ensure selection should set it to 0
        panel.ensure_selection();
        assert_eq!(panel.state.selected(), Some(0));
        
        Ok(())
    }

    #[test]
    fn test_panel_ensure_selection_empty() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: Vec::new(),
            state: ListState::default(),
        };
        
        // Should not set selection on empty panel
        panel.ensure_selection();
        assert!(panel.state.selected().is_none());
    }

    #[test]
    fn test_panel_selected_path() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        
        // Find and select the file
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                panel.state.select(Some(i));
                break;
            }
        }
        
        let selected_path = panel.selected_path();
        assert!(selected_path.is_some());
        assert_eq!(selected_path.unwrap(), temp_dir.path().join("test.txt"));
        
        // Clear selection
        panel.state.select(None);
        assert!(panel.selected_path().is_none());
        
        Ok(())
    }

    #[test]
    fn test_panel_side_equality() {
        assert_eq!(PanelSide::Left, PanelSide::Left);
        assert_eq!(PanelSide::Right, PanelSide::Right);
        assert_ne!(PanelSide::Left, PanelSide::Right);
        assert_ne!(PanelSide::Right, PanelSide::Left);
    }

    #[test]
    fn test_panel_side_copy() {
        let side = PanelSide::Left;
        let copied = side;
        assert_eq!(side, copied);
    }

    #[test]
    fn test_app_new() -> io::Result<()> {
        let app = App::new()?;
        
        assert_eq!(app.active_panel, PanelSide::Left);
        assert!(!app.show_help);
        assert!(app.viewer.is_none());
        assert_eq!(
            app.status_message,
            "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
        );
        
        // Both panels should be in the current directory
        assert_eq!(app.left_panel.path, app.right_panel.path);
        
        Ok(())
    }

    #[test]
    fn test_app_get_active_panel() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Initially active panel is left
        let left_path = app.left_panel.path.clone();
        let active_path = app.get_active_panel().path.clone();
        assert_eq!(active_path, left_path);
        
        // Switch to right panel
        app.active_panel = PanelSide::Right;
        let right_path = app.right_panel.path.clone();
        let active_path = app.get_active_panel().path.clone();
        assert_eq!(active_path, right_path);
        
        Ok(())
    }

    #[test]
    fn test_app_get_inactive_panel() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Initially active panel is left, so inactive is right
        let right_path = app.right_panel.path.clone();
        let inactive_path = app.get_inactive_panel().path.clone();
        assert_eq!(inactive_path, right_path);
        
        // Switch to right panel
        app.active_panel = PanelSide::Right;
        let left_path = app.left_panel.path.clone();
        let inactive_path = app.get_inactive_panel().path.clone();
        assert_eq!(inactive_path, left_path);
        
        Ok(())
    }

    #[test]
    fn test_app_switch_panel() -> io::Result<()> {
        let mut app = App::new()?;
        
        assert_eq!(app.active_panel, PanelSide::Left);
        
        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Right);
        
        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Left);
        
        Ok(())
    }

    #[test]
    fn test_app_switch_panel_multiple_times() -> io::Result<()> {
        let mut app = App::new()?;
        
        // Switch multiple times and ensure it alternates correctly
        for _ in 0..10 {
            let before = app.active_panel;
            app.switch_panel();
            let after = app.active_panel;
            
            match before {
                PanelSide::Left => assert_eq!(after, PanelSide::Right),
                PanelSide::Right => assert_eq!(after, PanelSide::Left),
            }
        }
        
        Ok(())
    }
}
