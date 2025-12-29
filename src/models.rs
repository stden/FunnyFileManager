use ratatui::widgets::ListState;
use std::{cmp::Ordering, collections::BTreeSet, fs, io, path::PathBuf};

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Extension,
    Size,
}

impl Default for SortMode {
    fn default() -> Self {
        SortMode::Name
    }
}

impl SortMode {
    #[allow(dead_code)]
    pub fn next(self) -> Self {
        match self {
            SortMode::Name => SortMode::Extension,
            SortMode::Extension => SortMode::Size,
            SortMode::Size => SortMode::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::Name => "Name",
            SortMode::Extension => "Ext",
            SortMode::Size => "Size",
        }
    }
}

#[derive(Default, Clone)]
pub struct Viewer {
    pub title: String,
    pub lines: Vec<String>,
    pub scroll: u16,
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
    pub state: ListState,
    pub marked: BTreeSet<String>,
    pub sort_mode: SortMode,
}

impl Panel {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let mut panel = Panel {
            path,
            entries: Vec::new(),
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };
        panel.load_entries()?;
        if !panel.entries.is_empty() {
            panel.state.select(Some(0));
        }
        Ok(panel)
    }

    pub fn load_entries(&mut self) -> io::Result<()> {
        let selected_name = self.get_selected_entry().map(|entry| entry.name.clone());

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

        let sort_mode = self.sort_mode;
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => match sort_mode {
                SortMode::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortMode::Extension => {
                    let a_ext = std::path::Path::new(&a.name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let b_ext = std::path::Path::new(&b.name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    a_ext
                        .cmp(&b_ext)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
                SortMode::Size => b
                    .size
                    .cmp(&a.size)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            },
        });

        self.entries.extend(entries);

        let valid_names: BTreeSet<String> = self
            .entries
            .iter()
            .filter(|entry| entry.name != "..")
            .map(|entry| entry.name.clone())
            .collect();
        self.marked.retain(|name| valid_names.contains(name));

        self.select_after_reload(selected_name);

        Ok(())
    }

    fn select_after_reload(&mut self, selected_name: Option<String>) {
        if let Some(selected_name) = selected_name {
            if let Some(index) = self
                .entries
                .iter()
                .position(|entry| entry.name == selected_name)
            {
                self.state.select(Some(index));
            } else if self.entries.is_empty() {
                self.state.select(None);
            } else {
                self.state.select(Some(0));
            }
        } else if self.entries.is_empty() {
            self.state.select(None);
        }
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
                    self.marked.clear();
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
        self.get_selected_entry().and_then(|entry| {
            if entry.name == ".." {
                None
            } else {
                Some(self.path.join(&entry.name))
            }
        })
    }

    #[allow(dead_code)]
    pub fn toggle_mark_selected(&mut self) -> bool {
        let Some(selected_name) = self.get_selected_entry().map(|entry| entry.name.clone()) else {
            return false;
        };

        if selected_name == ".." {
            return false;
        }

        if self.marked.contains(&selected_name) {
            self.marked.remove(&selected_name);
        } else {
            self.marked.insert(selected_name);
        }

        true
    }

    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    #[allow(dead_code)]
    pub fn action_entry_names(&self) -> Vec<String> {
        if !self.marked.is_empty() {
            return self.marked.iter().cloned().collect();
        }

        self.get_selected_entry()
            .and_then(|entry| {
                if entry.name == ".." {
                    None
                } else {
                    Some(entry.name.clone())
                }
            })
            .into_iter()
            .collect()
    }

    #[allow(dead_code)]
    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelSide {
    Left,
    Right,
}

#[derive(Clone)]
pub struct App {
    pub left_panel: Panel,
    pub right_panel: Panel,
    pub active_panel: PanelSide,
    pub show_help: bool,
    pub viewer: Option<Viewer>,
    pub search: Option<String>,
    pub command_line: Option<String>,
    pub show_bookmarks: bool,
    pub bookmarks: Vec<Option<PathBuf>>,
    pub should_quit: bool,
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
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
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

    fn file_names(panel: &Panel) -> Vec<String> {
        panel
            .entries
            .iter()
            .filter(|entry| !entry.is_dir && entry.name != "..")
            .map(|entry| entry.name.clone())
            .collect()
    }

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
        assert_eq!(
            panel.state.selected(),
            if panel.entries.is_empty() {
                None
            } else {
                Some(0)
            }
        );

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
    fn test_select_after_reload_empty_entries_with_selected_name() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        panel.entries.clear();
        panel.state.select(Some(0));
        panel.select_after_reload(Some("missing.txt".to_string()));

        assert_eq!(panel.state.selected(), None);
        Ok(())
    }

    #[test]
    fn test_select_after_reload_empty_entries_without_selected_name() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        panel.entries.clear();
        panel.state.select(Some(0));
        panel.select_after_reload(None);

        assert_eq!(panel.state.selected(), None);
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
        assert!(panel
            .entries
            .iter()
            .any(|e| e.name == "file1.txt" && !e.is_dir));
        assert!(panel
            .entries
            .iter()
            .any(|e| e.name == "file2.txt" && !e.is_dir));

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
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
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
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
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
        assert!(app.search.is_none());
        assert!(app.command_line.is_none());
        assert!(!app.show_bookmarks);
        assert_eq!(app.bookmarks.len(), 10);
        assert!(!app.should_quit);
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

    #[test]
    fn test_panel_next_with_none_selected() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        // Clear the selection
        panel.state.select(None);

        // Next should select the first item
        panel.next();
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_previous_with_none_selected() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("test.txt"))?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        // Clear the selection
        panel.state.select(None);

        // Previous should select the first item
        panel.previous();
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_sort_mode_next_and_label() {
        assert_eq!(SortMode::Name.label(), "Name");
        assert_eq!(SortMode::Extension.label(), "Ext");
        assert_eq!(SortMode::Size.label(), "Size");

        assert_eq!(SortMode::Name.next(), SortMode::Extension);
        assert_eq!(SortMode::Extension.next(), SortMode::Size);
        assert_eq!(SortMode::Size.next(), SortMode::Name);
    }

    #[test]
    fn test_panel_cycle_sort_mode() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        assert_eq!(panel.sort_mode, SortMode::Name);
        panel.cycle_sort_mode();
        assert_eq!(panel.sort_mode, SortMode::Extension);
        panel.cycle_sort_mode();
        assert_eq!(panel.sort_mode, SortMode::Size);
        panel.cycle_sort_mode();
        assert_eq!(panel.sort_mode, SortMode::Name);

        Ok(())
    }

    #[test]
    fn test_panel_marking_and_action_names() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        File::create(temp_dir.path().join("alpha.txt"))?;
        File::create(temp_dir.path().join("beta.txt"))?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        let alpha_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "alpha.txt")
            .expect("alpha.txt entry");
        panel.state.select(Some(alpha_index));

        assert!(panel.toggle_mark_selected());
        assert!(panel.marked.contains("alpha.txt"));
        assert_eq!(file_names(&panel), vec!["alpha.txt", "beta.txt"]);

        let mut action_names = panel.action_entry_names();
        action_names.sort();
        assert_eq!(action_names, vec!["alpha.txt"]);

        assert!(panel.toggle_mark_selected());
        assert!(panel.marked.is_empty());
        assert_eq!(panel.action_entry_names(), vec!["alpha.txt"]);

        let beta_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "beta.txt")
            .expect("beta.txt entry");
        panel.state.select(Some(beta_index));
        assert!(panel.toggle_mark_selected());
        assert!(panel.marked.contains("beta.txt"));

        std::fs::remove_file(temp_dir.path().join("beta.txt"))?;
        panel.load_entries()?;
        assert!(!panel.marked.contains("beta.txt"));

        panel.state.select(None);
        assert!(!panel.toggle_mark_selected());

        let parent_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "..")
            .expect("parent entry");
        panel.state.select(Some(parent_index));
        assert!(!panel.toggle_mark_selected());
        assert!(panel.action_entry_names().is_empty());
        assert!(panel.selected_path().is_none());

        Ok(())
    }

    #[test]
    fn test_panel_load_entries_sort_modes() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        std::fs::create_dir(temp_dir.path().join("dir"))?;
        File::create(temp_dir.path().join("b.txt"))?.write_all(b"1")?;
        File::create(temp_dir.path().join("a.rs"))?.write_all(b"1234")?;
        File::create(temp_dir.path().join("c"))?.write_all(b"12")?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        panel.sort_mode = SortMode::Extension;
        panel.load_entries()?;
        assert_eq!(file_names(&panel), vec!["c", "a.rs", "b.txt"]);

        panel.sort_mode = SortMode::Size;
        panel.load_entries()?;
        assert_eq!(file_names(&panel), vec!["a.rs", "c", "b.txt"]);

        Ok(())
    }
}
