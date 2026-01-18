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
        panel.state.select(panel.entries.first().map(|_| 0));
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
        let Some(i) = self.state.selected() else {
            return Ok(());
        };
        let Some(entry) = self.entries.get(i) else {
            return Ok(());
        };
        if !entry.is_dir {
            return Ok(());
        }

        if entry.name == ".." {
            self.path.pop();
        } else {
            self.path = self.path.join(&entry.name);
        }
        self.marked.clear();
        self.load_entries()?;
        self.state.select(self.entries.first().map(|_| 0));
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

    /// Find and select the next entry matching the given prefix (case-insensitive).
    /// If already on a matching entry, moves to the next matching one.
    /// Returns true if a match was found.
    pub fn find_next_matching(&mut self, prefix: &str) -> bool {
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        let prefix_lower = prefix.to_lowercase();
        let current = self.state.selected().unwrap_or(0);
        let len = self.entries.len();

        // Search from current+1 to end, then from 0 to current (inclusive)
        for offset in 1..=len {
            let idx = (current + offset) % len;
            let entry = &self.entries[idx];
            if entry.name != ".." && entry.name.to_lowercase().starts_with(&prefix_lower) {
                self.state.select(Some(idx));
                return true;
            }
        }

        false
    }

    /// Find and select the first entry matching the given prefix (case-insensitive).
    /// Returns true if a match was found.
    pub fn find_first_matching(&mut self, prefix: &str) -> bool {
        if self.entries.is_empty() || prefix.is_empty() {
            return false;
        }

        let prefix_lower = prefix.to_lowercase();

        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.name != ".." && entry.name.to_lowercase().starts_with(&prefix_lower) {
                self.state.select(Some(idx));
                return true;
            }
        }

        false
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
    pub quick_search: Option<String>,
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
            quick_search: None,
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
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        assert_eq!(panel.path, temp_dir.path());
        assert!(!panel.entries.is_empty());
        assert_eq!(panel.state.selected(), Some(0));

        panel.entries.clear();
        panel.state.select(None);
        assert!(panel.entries.is_empty());
        assert_eq!(panel.state.selected(), None);

        Ok(())
    }

    #[test]
    fn test_panel_load_entries_empty_dir() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        panel.load_entries().expect("test setup");

        assert!(panel.entries.iter().any(|e| e.name == ".."));

        Ok(())
    }

    #[test]
    fn test_select_after_reload_empty_entries_with_selected_name() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        panel.entries.clear();
        panel.state.select(Some(0));
        panel.select_after_reload(Some("missing.txt".to_string()));

        assert_eq!(panel.state.selected(), None);
        Ok(())
    }

    #[test]
    fn test_select_after_reload_empty_entries_without_selected_name() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        panel.entries.clear();
        panel.state.select(Some(0));
        panel.select_after_reload(None);

        assert_eq!(panel.state.selected(), None);
        Ok(())
    }

    #[test]
    fn test_select_after_reload_preserves_selected_name() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: vec![
                DirEntry {
                    name: "alpha.txt".to_string(),
                    is_dir: false,
                    size: 10,
                },
                DirEntry {
                    name: "beta.txt".to_string(),
                    is_dir: false,
                    size: 20,
                },
            ],
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };

        panel.state.select(Some(0));
        panel.select_after_reload(Some("beta.txt".to_string()));

        assert_eq!(panel.state.selected(), Some(1));
    }

    #[test]
    fn test_select_after_reload_selects_first_when_missing() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: vec![
                DirEntry {
                    name: "alpha.txt".to_string(),
                    is_dir: false,
                    size: 10,
                },
                DirEntry {
                    name: "beta.txt".to_string(),
                    is_dir: false,
                    size: 20,
                },
            ],
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };

        panel.state.select(Some(1));
        panel.select_after_reload(Some("missing.txt".to_string()));

        assert_eq!(panel.state.selected(), Some(0));
    }

    #[test]
    fn test_select_after_reload_keeps_selection_when_none() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: vec![
                DirEntry {
                    name: "alpha.txt".to_string(),
                    is_dir: false,
                    size: 10,
                },
                DirEntry {
                    name: "beta.txt".to_string(),
                    is_dir: false,
                    size: 20,
                },
            ],
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };

        panel.state.select(Some(1));
        panel.select_after_reload(None);

        assert_eq!(panel.state.selected(), Some(1));
    }

    #[test]
    fn test_panel_load_entries_preserves_selection() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("a.txt"))
            .expect("test setup")
            .write_all(b"a")
            .expect("test setup");
        File::create(temp_dir.path().join("b.txt"))
            .expect("test setup")
            .write_all(b"b")
            .expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        let selected_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "b.txt")
            .expect("b.txt entry");
        panel.state.select(Some(selected_index));

        File::create(temp_dir.path().join("aa.txt"))
            .expect("test setup")
            .write_all(b"aa")
            .expect("test setup");
        panel.load_entries().expect("test setup");

        let selected = panel.get_selected_entry().expect("selected entry");
        assert_eq!(selected.name, "b.txt");

        Ok(())
    }

    #[test]
    fn test_panel_load_entries_selects_first_when_selected_missing() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("a.txt"))
            .expect("test setup")
            .write_all(b"a")
            .expect("test setup");
        File::create(temp_dir.path().join("b.txt"))
            .expect("test setup")
            .write_all(b"b")
            .expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        let selected_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "b.txt")
            .expect("b.txt entry");
        panel.state.select(Some(selected_index));

        fs::remove_file(temp_dir.path().join("b.txt")).expect("test setup");
        panel.load_entries().expect("test setup");

        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_load_entries_with_files() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        // Create test files and directories
        fs::create_dir(temp_dir.path().join("dir1")).expect("test setup");
        fs::create_dir(temp_dir.path().join("dir2")).expect("test setup");
        File::create(temp_dir.path().join("file1.txt"))
            .expect("test setup")
            .write_all(b"test")
            .expect("test setup");
        File::create(temp_dir.path().join("file2.txt"))
            .expect("test setup")
            .write_all(b"test")
            .expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        // Check entries are sorted (directories first, then files)
        let mut found_file = false;
        let mut found_dir_after_file = false;
        for entry in &panel.entries {
            if entry.name != ".." {
                found_dir_after_file |= entry.is_dir && found_file;
                found_file |= !entry.is_dir;
            }
        }
        assert!(
            !found_dir_after_file,
            "Found directory after file in sorted entries"
        );

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
        let temp_dir = TempDir::new().expect("test setup");

        // Create multiple files
        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");

        // Create multiple files
        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Find and select the subdirectory
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                panel.state.select(Some(i));
                break;
            }
        }

        // Enter the directory
        panel.enter().expect("test setup");
        assert_eq!(panel.path, subdir);

        // Should have .. entry
        assert!(panel.entries.iter().any(|e| e.name == ".."));

        // Go back to parent
        panel.state.select(Some(0)); // Select ..
        panel.enter().expect("test setup");
        assert_eq!(panel.path, temp_dir.path());

        Ok(())
    }

    #[test]
    fn test_panel_enter_clears_marks() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).expect("test setup");
        File::create(temp_dir.path().join("file.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let file_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "file.txt")
            .expect("file.txt entry");
        panel.state.select(Some(file_index));
        assert!(panel.toggle_mark_selected());
        assert!(panel.marked.contains("file.txt"));

        let dir_index = panel
            .entries
            .iter()
            .position(|entry| entry.name == "subdir")
            .expect("subdir entry");
        panel.state.select(Some(dir_index));
        panel.enter().expect("test setup");

        assert!(panel.marked.is_empty());

        Ok(())
    }

    #[test]
    fn test_panel_enter_file() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        let original_path = panel.path.clone();

        // Find and select the file
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                panel.state.select(Some(i));
                break;
            }
        }

        // Entering a file should not change the path
        panel.enter().expect("test setup");
        assert_eq!(panel.path, original_path);

        Ok(())
    }

    #[test]
    fn test_panel_enter_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.state.select(None);

        panel.enter().expect("test setup");
        assert_eq!(panel.path, temp_dir.path());

        Ok(())
    }

    #[test]
    fn test_panel_enter_missing_entry() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.state.select(Some(10_000));

        panel.enter().expect("test setup");
        assert_eq!(panel.path, temp_dir.path());

        Ok(())
    }

    #[test]
    fn test_panel_get_selected_entry() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let app = App::new().expect("test setup");

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
        let mut app = App::new().expect("test setup");

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
        let mut app = App::new().expect("test setup");

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
        let mut app = App::new().expect("test setup");

        assert_eq!(app.active_panel, PanelSide::Left);

        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Right);

        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Left);

        Ok(())
    }

    #[test]
    fn test_app_switch_panel_multiple_times() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Clear the selection
        panel.state.select(None);

        // Next should select the first item
        panel.next();
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_previous_with_none_selected() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("test.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
        File::create(temp_dir.path().join("beta.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
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

        std::fs::remove_file(temp_dir.path().join("beta.txt")).expect("test setup");
        panel.load_entries().expect("test setup");
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
        let temp_dir = TempDir::new().expect("test setup");
        std::fs::create_dir(temp_dir.path().join("dir")).expect("test setup");
        File::create(temp_dir.path().join("b.txt"))
            .expect("test setup")
            .write_all(b"1")
            .expect("test setup");
        File::create(temp_dir.path().join("a.rs"))
            .expect("test setup")
            .write_all(b"1234")
            .expect("test setup");
        File::create(temp_dir.path().join("c"))
            .expect("test setup")
            .write_all(b"12")
            .expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        panel.sort_mode = SortMode::Extension;
        panel.load_entries().expect("test setup");
        assert_eq!(file_names(&panel), vec!["c", "a.rs", "b.txt"]);

        panel.sort_mode = SortMode::Size;
        panel.load_entries().expect("test setup");
        assert_eq!(file_names(&panel), vec!["a.rs", "c", "b.txt"]);

        Ok(())
    }

    #[test]
    fn test_panel_navigation_wraparound() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        for i in 0..3 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        // With 3 files plus ".." entry, we have indices 0-3
        panel.state.select(Some(0));
        panel.previous();
        assert_eq!(panel.state.selected(), Some(3));

        panel.state.select(Some(3));
        panel.next();
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_with_max_entries() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        for i in 0..1000 {
            File::create(temp_dir.path().join(format!("file{:04}.txt", i))).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        assert_eq!(panel.entries.len(), 1001);

        panel.state.select(Some(0));
        for _ in 0..500 {
            panel.next();
        }
        assert_eq!(panel.state.selected(), Some(500));

        panel.state.select(Some(1000));
        let entry_name = panel.entries[1000].name.clone();
        assert_eq!(panel.selected_path(), Some(panel.path.join(entry_name)));

        Ok(())
    }

    #[test]
    fn test_panel_empty_directory_operations() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let empty_dir = temp_dir.path().join("empty");
        fs::create_dir(&empty_dir).expect("test setup");

        let mut panel = Panel::new(empty_dir).expect("test setup");
        panel.load_entries().expect("test setup");

        assert_eq!(panel.entries.len(), 1);
        assert_eq!(panel.entries[0].name, "..");

        panel.next();
        assert_eq!(panel.state.selected(), Some(0));

        panel.previous();
        assert_eq!(panel.state.selected(), Some(0));

        panel.toggle_mark_selected();
        assert!(panel.marked.is_empty());

        Ok(())
    }

    #[test]
    fn test_panel_unicode_filenames() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let unicode_names = vec![
            "файл.txt",
            "文件.doc",
            "αρχείο.pdf",
            "🎵music🎵.mp3",
            "file with spaces.txt",
        ];

        for name in &unicode_names {
            File::create(temp_dir.path().join(name)).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        assert_eq!(panel.entries.len(), unicode_names.len() + 1);

        for name in &unicode_names {
            let found = panel.entries.iter().any(|e| e.name == *name);
            assert!(found, "Unicode filename {} not found", name);
        }

        Ok(())
    }

    #[test]
    fn test_panel_very_long_filenames() -> io::Result<()> {
        #[cfg(not(windows))]
        {
            let temp_dir = TempDir::new().expect("test setup");
            let long_name = "a".repeat(255);
            let file_path = temp_dir.path().join(&long_name);
            File::create(&file_path).expect("test setup");
            let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
            panel.load_entries().expect("test setup");

            let found = panel.entries.iter().any(|e| e.name == long_name);
            assert!(found);
        }
        #[cfg(windows)]
        {
            // Long filename test skipped on Windows
        }

        Ok(())
    }

    #[test]
    fn test_sort_mode_edge_cases() {
        let mut mode = SortMode::Name;

        mode = mode.next();
        assert_eq!(mode, SortMode::Extension);
        assert_eq!(mode.label(), "Ext");

        mode = mode.next();
        assert_eq!(mode, SortMode::Size);
        assert_eq!(mode.label(), "Size");

        mode = mode.next();
        assert_eq!(mode, SortMode::Name);
        assert_eq!(mode.label(), "Name");
    }

    #[test]
    fn test_app_bookmarks_boundary() -> io::Result<()> {
        let mut app = App::new().expect("test setup");

        assert_eq!(app.bookmarks.len(), 10);

        for (i, bookmark) in app.bookmarks.iter().enumerate() {
            assert!(bookmark.is_none(), "Bookmark {} should be None", i);
        }

        app.bookmarks[0] = Some(PathBuf::from("/test1"));
        app.bookmarks[9] = Some(PathBuf::from("/test2"));

        assert_eq!(app.bookmarks[0], Some(PathBuf::from("/test1")));
        assert_eq!(app.bookmarks[9], Some(PathBuf::from("/test2")));

        Ok(())
    }

    #[test]
    fn test_panel_marking_operations_edge_cases() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i))).expect("test setup");
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        panel.state.select(Some(1));
        panel.toggle_mark_selected();
        panel.toggle_mark_selected();
        assert!(panel.marked.is_empty());

        for i in 1..=5 {
            panel.state.select(Some(i));
            panel.toggle_mark_selected();
        }
        assert_eq!(panel.marked.len(), 5);

        panel.clear_marks();
        assert!(panel.marked.is_empty());

        panel.state.select(Some(0));
        panel.toggle_mark_selected();
        assert!(panel.marked.is_empty());

        Ok(())
    }

    #[test]
    fn test_sort_mode_cycle() {
        let mut mode = SortMode::Name;
        assert_eq!(mode, SortMode::Name);

        mode = mode.next();
        assert_eq!(mode, SortMode::Extension);

        mode = mode.next();
        assert_eq!(mode, SortMode::Size);

        mode = mode.next();
        assert_eq!(mode, SortMode::Name);
    }

    #[test]
    fn test_viewer_with_extreme_content() {
        let viewer = Viewer {
            lines: vec!["x".repeat(1000); 10000],
            scroll: 0,
            title: "Large".to_string(),
        };

        assert_eq!(viewer.lines.len(), 10000);
        assert_eq!(viewer.lines[0].len(), 1000);

        let empty_viewer = Viewer {
            lines: vec![],
            scroll: 0,
            title: "Empty".to_string(),
        };

        assert_eq!(empty_viewer.lines.len(), 0);
    }

    #[test]
    fn test_sort_mode_default() {
        // Test the Default implementation for SortMode
        let default_mode: SortMode = Default::default();
        assert_eq!(default_mode, SortMode::Name);
    }

    #[test]
    fn test_sort_mode_extension_no_extension() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        // Create files with and without extensions
        File::create(temp_dir.path().join("file1.txt")).expect("test setup");
        File::create(temp_dir.path().join("file2")).expect("test setup"); // No extension
        File::create(temp_dir.path().join("file3.md")).expect("test setup");
        File::create(temp_dir.path().join("file4")).expect("test setup"); // No extension

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.sort_mode = SortMode::Extension;
        panel.load_entries().expect("test setup");

        // Files without extensions should sort before files with extensions (after "..")
        let entries = &panel.entries;
        assert!(
            entries[0].name == ".." || entries[1].name == "file2" || entries[2].name == "file4"
        );

        Ok(())
    }

    #[test]
    fn test_toggle_mark_selected_no_selection() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Clear selection
        panel.state.select(None);

        // Should return false when no selection
        assert!(!panel.toggle_mark_selected());
        assert!(panel.marked.is_empty());

        Ok(())
    }

    #[test]
    fn test_panel_enter_parent_directory_navigation() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).expect("test setup");

        let mut panel = Panel::new(subdir).expect("test setup");
        panel.load_entries().expect("test setup");

        // Select parent directory ".."
        panel.state.select(Some(0));
        let selected_entry = panel.get_selected_entry().unwrap();
        assert_eq!(selected_entry.name, "..");

        // Enter parent directory
        panel.enter().expect("test setup");

        // Should now be in the parent directory
        assert_eq!(
            panel.path.canonicalize().expect("test setup"),
            temp_dir.path().canonicalize().expect("test setup")
        );

        Ok(())
    }

    #[test]
    fn test_action_entry_names_parent_directory() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        File::create(temp_dir.path().join("file.txt")).expect("test setup");
        panel.load_entries().expect("test setup");

        // Select parent directory ".."
        panel.state.select(Some(0));
        let names = panel.action_entry_names();

        // Should be empty when parent directory is selected and no marks
        assert!(names.is_empty());

        Ok(())
    }

    #[test]
    fn test_panel_action_entry_names_with_special_cases() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        File::create(temp_dir.path().join("regular.txt")).expect("test setup");
        File::create(temp_dir.path().join(".hidden")).expect("test setup");
        fs::create_dir(temp_dir.path().join("directory")).expect("test setup");

        panel.load_entries().expect("test setup");

        let names = panel.action_entry_names();
        assert!(names.is_empty());

        panel.state.select(Some(1));
        let names = panel.action_entry_names();
        assert_eq!(names.len(), 1);

        panel.marked.insert("regular.txt".to_string());
        panel.marked.insert(".hidden".to_string());
        panel.marked.insert("directory".to_string());

        let names = panel.action_entry_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"regular.txt".to_string()));
        assert!(names.contains(&".hidden".to_string()));
        assert!(names.contains(&"directory".to_string()));

        Ok(())
    }

    #[test]
    fn test_panel_load_entries_with_selected_name_edge_case() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Create files
        File::create(temp_dir.path().join("file1.txt")).expect("test setup");
        File::create(temp_dir.path().join("file2.txt")).expect("test setup");

        // Select file2.txt
        panel.load_entries().expect("test setup");
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "file2.txt" {
                panel.state.select(Some(i));
                break;
            }
        }

        // Reload should preserve selection of file2.txt
        panel.load_entries().expect("test setup");

        // Should still have file2.txt selected
        let idx = panel.state.selected().expect("selection");
        assert_eq!(panel.entries[idx].name, "file2.txt");

        // Now delete file2.txt and reload - should fall back to first entry
        fs::remove_file(temp_dir.path().join("file2.txt")).expect("test setup");
        panel.load_entries().expect("test setup");

        // Should fall back to selecting first entry since file2.txt doesn't exist
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_ensure_selection_corner_case() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Test empty entries with no selection
        panel.entries.clear();
        panel.state.select(None);
        panel.ensure_selection();
        assert_eq!(panel.state.selected(), None); // Should remain None for empty entries

        Ok(())
    }

    #[test]
    fn test_panel_sort_modes_with_mixed_case() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Create files with mixed case names
        File::create(temp_dir.path().join("UPPERCASE.txt")).expect("test setup");
        File::create(temp_dir.path().join("lowercase.txt")).expect("test setup");
        File::create(temp_dir.path().join("MixedCase.txt")).expect("test setup");
        fs::create_dir(temp_dir.path().join("Directory")).expect("test setup");

        // Test that directories come before files in all sort modes
        panel.sort_mode = SortMode::Name;
        panel.load_entries().expect("test setup");

        // Find positions of directory and files
        let dir_pos = panel
            .entries
            .iter()
            .position(|e| e.name == "Directory" && e.is_dir);
        let file_positions: Vec<_> = panel
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir && e.name != "..")
            .map(|(i, _)| i)
            .collect();

        let dir_idx = dir_pos.expect("Directory entry");
        for file_idx in &file_positions {
            assert!(dir_idx < *file_idx, "Directory should come before files");
        }

        Ok(())
    }

    #[test]
    fn test_quick_search_find_first_matching() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
        File::create(temp_dir.path().join("beta.txt")).expect("test setup");
        File::create(temp_dir.path().join("gamma.txt")).expect("test setup");
        File::create(temp_dir.path().join("Alpha_upper.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Find first entry starting with "a" (case-insensitive)
        assert!(panel.find_first_matching("a"));
        let selected = panel.get_selected_entry().unwrap();
        assert!(selected.name.to_lowercase().starts_with("a"));

        // Find first entry starting with "b"
        assert!(panel.find_first_matching("b"));
        let selected = panel.get_selected_entry().unwrap();
        assert_eq!(selected.name, "beta.txt");

        // Non-existent prefix
        assert!(!panel.find_first_matching("z"));

        // Empty prefix
        assert!(!panel.find_first_matching(""));

        Ok(())
    }

    #[test]
    fn test_quick_search_find_next_matching() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("apple.txt")).expect("test setup");
        File::create(temp_dir.path().join("apricot.txt")).expect("test setup");
        File::create(temp_dir.path().join("avocado.txt")).expect("test setup");
        File::create(temp_dir.path().join("banana.txt")).expect("test setup");

        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Find first "a" entry
        assert!(panel.find_first_matching("a"));
        let first_match = panel.get_selected_entry().unwrap().name.clone();

        // Find next "a" entry - should cycle to a different one
        assert!(panel.find_next_matching("a"));
        let second_match = panel.get_selected_entry().unwrap().name.clone();
        assert_ne!(first_match, second_match);
        assert!(second_match.to_lowercase().starts_with("a"));

        Ok(())
    }

    #[test]
    fn test_quick_search_empty_panel() {
        let mut panel = Panel {
            path: PathBuf::new(),
            entries: Vec::new(),
            state: ListState::default(),
            marked: BTreeSet::new(),
            sort_mode: SortMode::default(),
        };

        assert!(!panel.find_first_matching("a"));
        assert!(!panel.find_next_matching("a"));
    }

    #[test]
    fn test_app_quick_search_field() -> io::Result<()> {
        let app = App::new().expect("test setup");
        assert!(app.quick_search.is_none());
        Ok(())
    }
}
