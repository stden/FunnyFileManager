#[cfg(test)]
mod mutation_killer_tests {
    use crate::models::App;
    use crate::utils::{format_size, unique_destination, copy_dir_all, refresh_panels};
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_format_size_mutations() {
        // Kill "replace format_size -> String with String::new()"
        let result = format_size(1000);
        assert!(!result.is_empty());
        assert!(result.len() > 0);
        
        // Kill "replace format_size -> String with 'xyzzy'.into()"
        assert!(!format_size(1000).contains("xyzzy"));
        
        // Kill "replace * with + in KB calculation"
        // KB = 1024, not 1024 + 1024
        assert_eq!(format_size(1024), "1.00 KB");
        assert_ne!(format_size(1024), format_size(2048));
        
        // Kill "replace * with / in MB calculation"
        // MB = KB * 1024, not KB / 1024
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_ne!(format_size(1048576), format_size(1));
        
        // Kill "replace >= with < in GB check"
        assert_eq!(format_size(1073741823), "1024.00 MB"); // Just below GB
        assert_eq!(format_size(1073741824), "1.00 GB"); // Exactly GB
        assert_ne!(format_size(1073741823), format_size(1073741824));
        
        // Kill "replace >= with < in MB check"
        assert_eq!(format_size(1048575), "1024.00 KB"); // Just below MB
        assert_eq!(format_size(1048576), "1.00 MB"); // Exactly MB
        assert_ne!(format_size(1048575), format_size(1048576));
        
        // Kill "replace >= with < in KB check"
        assert_eq!(format_size(1023), "1023 B"); // Just below KB
        assert_eq!(format_size(1024), "1.00 KB"); // Exactly KB
        assert_ne!(format_size(1023), format_size(1024));
    }

    #[test]
    fn test_unique_destination_mutations() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Kill "replace unique_destination -> PathBuf with Default::default()"
        let result = unique_destination(temp_dir.path(), OsStr::new("test.txt"));
        assert!(!result.as_os_str().is_empty());
        assert_ne!(result, PathBuf::default());
        
        // Kill mutations in match guard for extension handling
        // Test file with empty extension
        File::create(temp_dir.path().join("empty."))?;
        let unique = unique_destination(temp_dir.path(), OsStr::new("empty."));
        assert!(unique.to_string_lossy().contains("empty_1"));
        assert!(!unique.to_string_lossy().contains("empty_1.")); // No dot for empty extension
        
        // Test file with non-empty extension
        File::create(temp_dir.path().join("test.rs"))?;
        let unique = unique_destination(temp_dir.path(), OsStr::new("test.rs"));
        assert!(unique.to_string_lossy().contains("test_1.rs"));
        assert!(unique.to_string_lossy().contains(".rs")); // Extension preserved
        
        // Kill "replace += with -= in counter increment"
        // Create multiple conflicting files
        for i in 0..5 {
            if i == 0 {
                File::create(temp_dir.path().join("conflict.txt"))?;
            } else {
                File::create(temp_dir.path().join(format!("conflict_{}.txt", i)))?;
            }
        }
        let unique = unique_destination(temp_dir.path(), OsStr::new("conflict.txt"));
        assert!(unique.to_string_lossy().contains("conflict_5.txt"));
        // Verify it's not using negative numbers
        assert!(!unique.to_string_lossy().contains("-"));
        
        // Kill "replace += with *= in counter increment"
        // Verify sequential numbering, not exponential
        File::create(temp_dir.path().join("seq.txt"))?;
        File::create(temp_dir.path().join("seq_1.txt"))?;
        File::create(temp_dir.path().join("seq_2.txt"))?;
        let unique = unique_destination(temp_dir.path(), OsStr::new("seq.txt"));
        assert!(unique.to_string_lossy().contains("seq_3.txt"));
        assert!(!unique.to_string_lossy().contains("seq_4.txt")); // Not multiplied
        
        Ok(())
    }

    #[test]
    fn test_copy_dir_all_mutations() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");
        
        fs::create_dir(&src)?;
        File::create(src.join("test.txt"))?.write_all(b"content")?;
        
        // Kill "replace copy_dir_all -> io::Result<()> with Ok(())"
        copy_dir_all(&src, &dst)?;
        
        // Verify the copy actually happened
        assert!(dst.exists());
        assert!(dst.is_dir());
        assert!(dst.join("test.txt").exists());
        assert_eq!(fs::read_to_string(dst.join("test.txt"))?, "content");
        
        // Test nested directories
        fs::create_dir(src.join("subdir"))?;
        File::create(src.join("subdir").join("nested.txt"))?.write_all(b"nested")?;
        
        let dst2 = temp_dir.path().join("dst2");
        copy_dir_all(&src, &dst2)?;
        
        assert!(dst2.join("subdir").join("nested.txt").exists());
        assert_eq!(fs::read_to_string(dst2.join("subdir").join("nested.txt"))?, "nested");
        
        Ok(())
    }

    #[test]
    fn test_refresh_panels_mutations() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut app = App::new()?;
        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = temp_dir.path().to_path_buf();
        
        // Create initial state
        File::create(temp_dir.path().join("initial.txt"))?;
        app.left_panel.load_entries()?;
        app.right_panel.load_entries()?;
        let initial_left_count = app.left_panel.entries.len();
        let initial_right_count = app.right_panel.entries.len();
        
        // Add new files
        File::create(temp_dir.path().join("new1.txt"))?;
        File::create(temp_dir.path().join("new2.txt"))?;
        fs::create_dir(temp_dir.path().join("newdir"))?;
        
        // Kill "replace refresh_panels -> io::Result<()> with Ok(())"
        refresh_panels(&mut app)?;
        
        // Verify panels were actually refreshed
        assert!(app.left_panel.entries.len() > initial_left_count);
        assert!(app.right_panel.entries.len() > initial_right_count);
        
        // Verify new entries are present
        assert!(app.left_panel.entries.iter().any(|e| e.name == "new1.txt"));
        assert!(app.left_panel.entries.iter().any(|e| e.name == "new2.txt"));
        assert!(app.left_panel.entries.iter().any(|e| e.name == "newdir"));
        
        assert!(app.right_panel.entries.iter().any(|e| e.name == "new1.txt"));
        assert!(app.right_panel.entries.iter().any(|e| e.name == "new2.txt"));
        assert!(app.right_panel.entries.iter().any(|e| e.name == "newdir"));
        
        // Verify selections are maintained
        assert!(app.left_panel.state.selected().is_some());
        assert!(app.right_panel.state.selected().is_some());
        
        Ok(())
    }
}