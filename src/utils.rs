use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::App;

pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        return format!("{:.2} GB", size as f64 / GB as f64);
    }

    if size >= MB {
        return format!("{:.2} MB", size as f64 / MB as f64);
    }

    if size >= KB {
        return format!("{:.2} KB", size as f64 / KB as f64);
    }

    let mut bytes = size.to_string();
    bytes.push_str(" B");
    bytes
}

pub fn unique_destination(base: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let mut dest = base.join(file_name);
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let extension = Path::new(file_name).extension().and_then(|e| e.to_str());

    let mut counter = 1;
    while dest.exists() {
        let new_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{}_{}.{}", stem, counter, ext),
            _ => format!("{}_{}", stem, counter),
        };
        dest = base.join(new_name);
        counter += 1;
    }
    dest
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn refresh_panels(app: &mut App) -> io::Result<()> {
    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{App, Panel, PanelSide};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(999), "999 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(2048), "2.00 KB");
        assert_eq!(format_size(1048575), "1024.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1572864), "1.50 MB");
        assert_eq!(format_size(2097152), "2.00 MB");
        assert_eq!(format_size(1073741823), "1024.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(1610612736), "1.50 GB");
        assert_eq!(format_size(2147483648), "2.00 GB");
        assert_eq!(format_size(10737418240), "10.00 GB");
    }

    #[test]
    fn test_unique_destination_no_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let file_name = std::ffi::OsStr::new("test.txt");

        let dest = unique_destination(base, file_name);
        assert_eq!(dest, base.join("test.txt"));
    }

    #[test]
    fn test_unique_destination_with_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create existing files
        File::create(base.join("test.txt")).unwrap();
        File::create(base.join("test_1.txt")).unwrap();

        let file_name = std::ffi::OsStr::new("test.txt");
        let dest = unique_destination(base, file_name);
        assert_eq!(dest, base.join("test_2.txt"));
    }

    #[test]
    fn test_unique_destination_no_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        File::create(base.join("test")).unwrap();

        let file_name = std::ffi::OsStr::new("test");
        let dest = unique_destination(base, file_name);
        assert_eq!(dest, base.join("test_1"));
    }

    #[test]
    fn test_unique_destination_multiple_dots() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        File::create(base.join("test.backup.txt")).unwrap();

        let file_name = std::ffi::OsStr::new("test.backup.txt");
        let dest = unique_destination(base, file_name);
        assert_eq!(dest, base.join("test.backup_1.txt"));
    }

    #[test]
    fn test_unique_destination_empty_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        File::create(base.join("test.")).unwrap();

        let file_name = std::ffi::OsStr::new("test.");
        let dest = unique_destination(base, file_name);
        // When extension is empty, it's treated as no extension
        assert_eq!(dest, base.join("test_1"));
    }

    #[test]
    fn test_copy_dir_all_simple() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src)?;
        File::create(src.join("file1.txt"))?.write_all(b"content1")?;
        File::create(src.join("file2.txt"))?.write_all(b"content2")?;

        copy_dir_all(&src, &dst)?;

        assert!(dst.exists());
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("file2.txt").exists());

        let content1 = fs::read_to_string(dst.join("file1.txt"))?;
        assert_eq!(content1, "content1");

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_nested() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src)?;
        fs::create_dir(src.join("subdir1"))?;
        fs::create_dir(src.join("subdir1").join("subsubdir"))?;
        fs::create_dir(src.join("subdir2"))?;

        File::create(src.join("root.txt"))?.write_all(b"root")?;
        File::create(src.join("subdir1").join("file1.txt"))?.write_all(b"sub1")?;
        File::create(src.join("subdir1").join("subsubdir").join("deep.txt"))?.write_all(b"deep")?;
        File::create(src.join("subdir2").join("file2.txt"))?.write_all(b"sub2")?;

        copy_dir_all(&src, &dst)?;

        assert!(dst.exists());
        assert!(dst.join("root.txt").exists());
        assert!(dst.join("subdir1").join("file1.txt").exists());
        assert!(dst
            .join("subdir1")
            .join("subsubdir")
            .join("deep.txt")
            .exists());
        assert!(dst.join("subdir2").join("file2.txt").exists());

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_empty_dir() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src)?;

        copy_dir_all(&src, &dst)?;

        assert!(dst.exists());
        assert!(dst.is_dir());
        assert_eq!(fs::read_dir(&dst)?.count(), 0);

        Ok(())
    }

    #[test]
    fn test_refresh_panels() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        // Create initial files
        File::create(temp_dir.path().join("initial.txt"))?;

        let mut app = App {
            left_panel: Panel::new(temp_dir.path().to_path_buf())?,
            right_panel: Panel::new(temp_dir.path().to_path_buf())?,
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
        };

        // Add new files after app creation
        File::create(temp_dir.path().join("new_file.txt"))?;
        fs::create_dir(temp_dir.path().join("new_dir"))?;

        // Refresh panels
        refresh_panels(&mut app)?;

        // Check both panels contain the new entries
        let left_has_new_file = app
            .left_panel
            .entries
            .iter()
            .any(|e| e.name == "new_file.txt");
        let left_has_new_dir = app.left_panel.entries.iter().any(|e| e.name == "new_dir");
        let right_has_new_file = app
            .right_panel
            .entries
            .iter()
            .any(|e| e.name == "new_file.txt");
        let right_has_new_dir = app.right_panel.entries.iter().any(|e| e.name == "new_dir");

        assert!(left_has_new_file);
        assert!(left_has_new_dir);
        assert!(right_has_new_file);
        assert!(right_has_new_dir);

        Ok(())
    }

    #[test]
    fn test_refresh_panels_maintains_selection() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        // Create files
        File::create(temp_dir.path().join("a.txt"))?;
        File::create(temp_dir.path().join("b.txt"))?;
        File::create(temp_dir.path().join("c.txt"))?;

        let mut app = App {
            left_panel: Panel::new(temp_dir.path().to_path_buf())?,
            right_panel: Panel::new(temp_dir.path().to_path_buf())?,
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
        };

        // Set selection to second file
        app.left_panel.state.select(Some(2));
        app.right_panel.state.select(Some(1));

        // Refresh panels
        refresh_panels(&mut app)?;

        // Check selections are maintained
        assert!(app.left_panel.state.selected().is_some());
        assert!(app.right_panel.state.selected().is_some());

        Ok(())
    }
}
