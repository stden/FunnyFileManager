use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::App;

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_CREATE: u8 = 1;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_ENTRY: u8 = 2;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_FILE_TYPE: u8 = 3;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_RECURSE: u8 = 4;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_COPY_FILE: u8 = 5;

#[cfg(any(test, tarpaulin))]
static FORCE_REFRESH_ERROR: AtomicBool = AtomicBool::new(false);

#[cfg(any(test, tarpaulin))]
static FORCE_COPY_DIR_ALL_STEP: AtomicU8 = AtomicU8::new(0);

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

fn create_dir_all_checked(dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == COPY_DIR_ALL_FAIL_CREATE {
        FORCE_COPY_DIR_ALL_STEP.store(0, Ordering::SeqCst);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced create_dir_all failure",
        ));
    }
    fs::create_dir_all(dst)
}

fn read_dir_entry(entry: io::Result<fs::DirEntry>) -> io::Result<fs::DirEntry> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == COPY_DIR_ALL_FAIL_ENTRY {
        FORCE_COPY_DIR_ALL_STEP.store(0, Ordering::SeqCst);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced read_dir entry failure",
        ));
    }
    entry
}

fn entry_file_type(entry: &fs::DirEntry) -> io::Result<fs::FileType> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == COPY_DIR_ALL_FAIL_FILE_TYPE {
        FORCE_COPY_DIR_ALL_STEP.store(0, Ordering::SeqCst);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced file_type failure",
        ));
    }
    entry.file_type()
}

fn copy_file_checked(src: &Path, dst: &Path) -> io::Result<u64> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == COPY_DIR_ALL_FAIL_COPY_FILE {
        FORCE_COPY_DIR_ALL_STEP.store(0, Ordering::SeqCst);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced copy failure",
        ));
    }
    fs::copy(src, dst)
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == COPY_DIR_ALL_FAIL_RECURSE
        && src
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".ffm_force_copy_dir_error")
    {
        FORCE_COPY_DIR_ALL_STEP.store(0, Ordering::SeqCst);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced recursive copy failure",
        ));
    }

    create_dir_all_checked(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = read_dir_entry(entry)?;
        let file_type = entry_file_type(&entry)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            copy_file_checked(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn refresh_panels(app: &mut App) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_REFRESH_ERROR.load(Ordering::SeqCst) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "forced refresh_panels failure",
        ));
    }
    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

// These functions are defined but not currently used in tests
// They can be uncommented if needed for future mutation testing
// #[cfg(test)]
// pub(crate) fn set_copy_dir_all_step(step: u8) {
//     FORCE_COPY_DIR_ALL_STEP.store(step, Ordering::SeqCst);
// }
//
// #[cfg(test)]
// pub(crate) fn set_refresh_panels_error(value: bool) {
//     FORCE_REFRESH_ERROR.store(value, Ordering::SeqCst);
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{App, Panel, PanelSide};
    use std::ffi::OsStr;
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
    fn test_unique_destination_dotfile() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        File::create(base.join(".env")).unwrap();

        let file_name = std::ffi::OsStr::new(".env");
        let dest = unique_destination(base, file_name);
        assert_eq!(dest, base.join(".env_1"));
    }

    #[test]
    fn test_copy_dir_all_mutation_killer() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        File::create(src.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        // Test that function doesn't just return Ok(()) without doing anything
        copy_dir_all(&src, &dst).expect("test setup");

        // Verify the copy actually happened
        assert!(dst.exists());
        assert!(dst.join("file.txt").exists());
        assert_eq!(
            fs::read_to_string(dst.join("file.txt")).expect("test setup"),
            "content"
        );

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_src_not_exist() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("non_existent_src");
        let dst = temp_dir.path().join("dst");

        let result = copy_dir_all(&src, &dst);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_refresh_panels_mutation_killer() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = temp_dir.path().to_path_buf();

        // Create initial state
        app.left_panel.load_entries().expect("test setup");
        app.right_panel.load_entries().expect("test setup");
        let initial_count = app.left_panel.entries.len();

        // Add a new file
        File::create(temp_dir.path().join("new_file.txt")).expect("test setup");

        // Test that refresh_panels actually does something
        refresh_panels(&mut app).expect("test setup");

        // Verify panels were actually refreshed
        assert!(app.left_panel.entries.len() > initial_count);
        assert!(app.right_panel.entries.len() > initial_count);
        assert!(app
            .left_panel
            .entries
            .iter()
            .any(|e| e.name == "new_file.txt"));

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_simple() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        File::create(src.join("file1.txt"))
            .expect("test setup")
            .write_all(b"content1")
            .expect("test setup");
        File::create(src.join("file2.txt"))
            .expect("test setup")
            .write_all(b"content2")
            .expect("test setup");

        copy_dir_all(&src, &dst).expect("test setup");

        assert!(dst.exists());
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("file2.txt").exists());

        let content1 = fs::read_to_string(dst.join("file1.txt")).expect("test setup");
        assert_eq!(content1, "content1");

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_nested() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        fs::create_dir(src.join("subdir1")).expect("test setup");
        fs::create_dir(src.join("subdir1").join("subsubdir")).expect("test setup");
        fs::create_dir(src.join("subdir2")).expect("test setup");

        File::create(src.join("root.txt"))
            .expect("test setup")
            .write_all(b"root")
            .expect("test setup");
        File::create(src.join("subdir1").join("file1.txt"))
            .expect("test setup")
            .write_all(b"sub1")
            .expect("test setup");
        File::create(src.join("subdir1").join("subsubdir").join("deep.txt"))
            .expect("test setup")
            .write_all(b"deep")
            .expect("test setup");
        File::create(src.join("subdir2").join("file2.txt"))
            .expect("test setup")
            .write_all(b"sub2")
            .expect("test setup");

        copy_dir_all(&src, &dst).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");

        copy_dir_all(&src, &dst).expect("test setup");

        assert!(dst.exists());
        assert!(dst.is_dir());
        assert_eq!(fs::read_dir(&dst).expect("test setup").count(), 0);

        Ok(())
    }

    #[test]
    fn test_refresh_panels() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        // Create initial files
        File::create(temp_dir.path().join("initial.txt")).expect("test setup");

        let mut app = App {
            left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
            quick_search: None,
            show_drives: None,
            new_file_input: None,
        };

        // Add new files after app creation
        File::create(temp_dir.path().join("new_file.txt")).expect("test setup");
        fs::create_dir(temp_dir.path().join("new_dir")).expect("test setup");

        // Refresh panels
        refresh_panels(&mut app).expect("test setup");

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
        let temp_dir = TempDir::new().expect("test setup");

        // Create files
        File::create(temp_dir.path().join("a.txt")).expect("test setup");
        File::create(temp_dir.path().join("b.txt")).expect("test setup");
        File::create(temp_dir.path().join("c.txt")).expect("test setup");

        let mut app = App {
            left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
            quick_search: None,
            show_drives: None,
            new_file_input: None,
        };

        // Set selection to second file
        app.left_panel.state.select(Some(2));
        app.right_panel.state.select(Some(1));

        // Refresh panels
        refresh_panels(&mut app).expect("test setup");

        // Check selections are maintained
        assert!(app.left_panel.state.selected().is_some());
        assert!(app.right_panel.state.selected().is_some());

        Ok(())
    }

    #[test]
    fn test_refresh_panels_sets_selection_when_none() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        File::create(temp_dir.path().join("only.txt")).expect("test setup");

        let mut app = App {
            left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
            active_panel: PanelSide::Left,
            status_message: String::new(),
            show_help: false,
            viewer: None,
            search: None,
            command_line: None,
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            should_quit: false,
            quick_search: None,
            show_drives: None,
            new_file_input: None,
        };

        app.left_panel.state.select(None);
        app.right_panel.state.select(None);

        refresh_panels(&mut app).expect("test setup");

        assert!(app.left_panel.state.selected().is_some());
        assert!(app.right_panel.state.selected().is_some());

        Ok(())
    }

    #[test]
    fn test_format_size_edge_cases() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048575), "1024.00 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741823), "1024.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(u64::MAX), "17179869184.00 GB");
    }

    #[test]
    fn test_unique_destination_mutation_killer() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        // Test that kills "replace with Default::default()" mutation
        let result = unique_destination(temp_dir.path(), OsStr::new("test.txt"));
        assert!(!result.as_os_str().is_empty());
        assert!(result.starts_with(temp_dir.path()));

        // Test match guard mutations for extension handling
        let file_with_ext = temp_dir.path().join("file.txt");
        File::create(&file_with_ext).expect("test setup");
        let unique = unique_destination(temp_dir.path(), OsStr::new("file.txt"));
        assert!(unique.to_string_lossy().contains("file_1.txt"));

        // Test empty extension branch
        let file_no_ext = temp_dir.path().join("file");
        File::create(&file_no_ext).expect("test setup");
        let unique = unique_destination(temp_dir.path(), OsStr::new("file"));
        assert!(unique.to_string_lossy().contains("file_1"));
        assert!(!unique.to_string_lossy().ends_with(".")); // No trailing dot

        // Test counter increment mutations (+= vs -= vs *=)
        File::create(temp_dir.path().join("count.txt")).expect("test setup");
        for i in 1..5 {
            File::create(temp_dir.path().join(format!("count_{}.txt", i))).expect("test setup");
        }
        let unique = unique_destination(temp_dir.path(), OsStr::new("count.txt"));
        // Should be count_5.txt, not count_-1.txt or count_0.txt
        assert!(unique.to_string_lossy().contains("count_5.txt"));
        assert!(!unique.to_string_lossy().contains("count_-"));
        assert!(!unique.to_string_lossy().contains("count_0"));

        Ok(())
    }

    #[test]
    fn test_unique_destination_special_cases() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");

        let without_ext = temp_dir.path().join("noext");
        File::create(&without_ext).expect("test setup");
        let unique = unique_destination(temp_dir.path(), without_ext.file_name().unwrap());
        assert!(unique.to_string_lossy().contains("noext_1"));

        let multiple_dots = temp_dir.path().join("file.tar.gz");
        File::create(&multiple_dots).expect("test setup");
        let unique = unique_destination(temp_dir.path(), multiple_dots.file_name().unwrap());
        assert!(unique.to_string_lossy().contains("file.tar_1.gz"));

        let hidden = temp_dir.path().join(".hidden");
        File::create(&hidden).expect("test setup");
        let unique = unique_destination(temp_dir.path(), hidden.file_name().unwrap());
        assert!(unique.to_string_lossy().contains(".hidden_1"));

        let trailing_dot = temp_dir.path().join("file.");
        File::create(&trailing_dot).expect("test setup");
        let unique = unique_destination(temp_dir.path(), trailing_dot.file_name().unwrap());
        assert!(unique.to_string_lossy().contains("file_1"));

        let base_name = temp_dir.path().join("test");
        for i in 0..15 {
            if i == 0 {
                File::create(&base_name).expect("test setup");
            } else {
                File::create(temp_dir.path().join(format!("test_{}", i))).expect("test setup");
            }
        }
        let unique = unique_destination(temp_dir.path(), base_name.file_name().unwrap());
        assert!(unique.to_string_lossy().contains("test_15"));

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_with_symlinks() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        let file_path = src.join("file.txt");
        File::create(&file_path)
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = src.join("link.txt");
            symlink(&file_path, &link_path).expect("test setup");

            copy_dir_all(&src, &dst).expect("test setup");

            assert!(dst.join("file.txt").exists());
            assert!(dst.join("link.txt").exists());
        }

        #[cfg(not(unix))]
        {
            copy_dir_all(&src, &dst).expect("test setup");
            assert!(dst.join("file.txt").exists());
        }

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_permission_errors() -> io::Result<()> {
        #[cfg(unix)]
        {
            let temp_dir = TempDir::new().expect("test setup");
            let src = temp_dir.path().join("src");
            let dst = temp_dir.path().join("dst");

            fs::create_dir(&src).expect("test setup");
            File::create(src.join("file.txt"))
                .expect("test setup")
                .write_all(b"content")
                .expect("test setup");

            use std::os::unix::fs::PermissionsExt;
            fs::create_dir(&dst).expect("test setup");
            let mut perms = fs::metadata(&dst).expect("test setup").permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&dst, perms).expect("test setup");

            let result = copy_dir_all(&src, &dst);

            let mut perms = fs::metadata(&dst).expect("test setup").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dst, perms).expect("test setup");

            assert!(result.is_err());
        }

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_deeply_nested() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        let mut current = src.clone();
        for i in 0..10 {
            current = current.join(format!("level{}", i));
            fs::create_dir_all(&current).expect("test setup");
            File::create(current.join("file.txt"))
                .expect("test setup")
                .write_all(b"content")
                .expect("test setup");
        }

        copy_dir_all(&src, &dst).expect("test setup");

        let mut check = dst.clone();
        for i in 0..10 {
            check = check.join(format!("level{}", i));
            assert!(check.exists());
            assert!(check.join("file.txt").exists());
        }

        Ok(())
    }

    #[test]
    fn test_copy_dir_all_with_special_filenames() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");

        let special_names = vec![
            "file with spaces.txt",
            "file[brackets].txt",
            "file(parens).txt",
            "file{braces}.txt",
            ".hidden",
            "ñoño.txt",
            "файл.txt",
        ];

        for name in &special_names {
            File::create(src.join(name))
                .expect("test setup")
                .write_all(b"content")
                .expect("test setup");
        }

        copy_dir_all(&src, &dst).expect("test setup");

        for name in &special_names {
            assert!(dst.join(name).exists(), "File {} not copied", name);
        }

        Ok(())
    }

    #[test]
    fn test_refresh_panels_with_deleted_directories() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");

        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        let mut app = App {
            left_panel: Panel::new(left_dir.clone()).expect("test setup"),
            right_panel: Panel::new(right_dir.clone()).expect("test setup"),
            active_panel: PanelSide::Left,
            should_quit: false,
            viewer: None,
            show_help: false,
            search: None,
            command_line: None,
            status_message: String::new(),
            show_bookmarks: false,
            bookmarks: vec![None; 10],
            quick_search: None,
            show_drives: None,
            new_file_input: None,
        };

        fs::remove_dir(&left_dir).expect("test setup");
        fs::remove_dir(&right_dir).expect("test setup");

        let result = refresh_panels(&mut app);

        assert!(result.is_err() || app.left_panel.path != left_dir);

        Ok(())
    }

    #[test]
    fn test_format_size_mutation_killer() {
        // Test that kills mutations that replace function with String::new() or "xyzzy"
        assert!(!format_size(1000).is_empty());
        assert!(!format_size(1000).contains("xyzzy"));

        // Test exact KB constant (1024, not 1024 + 0 or 1024 / 1)
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1023), "1023 B");
        assert_ne!(format_size(1025), "1025 B"); // Should be KB

        // Test exact MB constant (1024 * 1024, not 1024 + 1024)
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1048575), "1024.00 KB");
        assert_ne!(format_size(1048577), "1048577 B"); // Should be MB not bytes

        // Test exact GB constant
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(1073741823), "1024.00 MB");

        // Test >= vs < mutations
        assert_ne!(format_size(1024), format_size(1023)); // KB boundary
        assert_ne!(format_size(1048576), format_size(1048575)); // MB boundary
        assert_ne!(format_size(1073741824), format_size(1073741823)); // GB boundary
    }

    #[test]
    fn test_format_size_precision() {
        assert_eq!(format_size(1500), "1.46 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1999), "1.95 KB");
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.50 MB");
        assert_eq!(
            format_size(1024 * 1024 * 1024 + 512 * 1024 * 1024),
            "1.50 GB"
        );
    }
}
