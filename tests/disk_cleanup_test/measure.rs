//! Purpose: Measuring candidates: relative paths, other machines, temp, heap dumps and caches.

use super::*;

#[test]
fn measure_resolves_relative_paths_against_the_base() {
    let temp = TempDir::new().expect("test setup");
    let target = temp.path().join("target");
    fs::create_dir(&target).expect("test setup");
    fs::write(target.join("artifact.bin"), vec![0_u8; 512]).expect("test setup");

    let mut dialog = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert_eq!(
        measured.path, target,
        "the relative path is stored back as an absolute one"
    );
    assert!(measured.exists);
    assert_eq!(measured.size.map(FileSize::bytes), Some(512));
    assert!(
        !measured.checked,
        "only the Temp folder is pre-ticked; everything else is opt-in"
    );
}

#[test]
fn measure_leaves_a_candidate_that_is_not_on_this_machine_unsized() {
    let temp = TempDir::new().expect("test setup");
    let absent = temp.path().join("nothing-here");

    let mut dialog = dialog_with(vec![candidate("Docker cache", absent.clone())]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert!(!measured.exists);
    assert_eq!(measured.size, None);
    assert!(!measured.checked);
    assert_eq!(measured.path, absent, "an absolute path is left alone");
}

#[test]
fn measure_pre_ticks_the_temp_folder() {
    let temp = TempDir::new().expect("test setup");
    let cache = temp.path().join("tmp-cache");
    fs::create_dir(&cache).expect("test setup");
    fs::write(cache.join("stale.log"), b"stale").expect("test setup");

    let row = CleanupCandidate {
        kind: CleanupKind::Contents,
        ..candidate("Temp files", cache)
    };
    let mut dialog = dialog_with(vec![row]);
    disk_cleanup::measure(&mut dialog, temp.path());

    assert!(
        dialog.candidates[0].checked,
        "emptying the Temp folder keeps the folder and other users' files, \
         so it starts ticked"
    );
    assert_eq!(dialog.selected_bytes(), 5);
}

#[test]
fn measure_sizes_only_the_heap_dumps_inside_the_home_directory() {
    let temp = TempDir::new().expect("test setup");
    let home = temp.path().join("home");
    fs::create_dir(&home).expect("test setup");
    fs::write(home.join("java_pid1.hprof"), vec![0_u8; 100]).expect("test setup");
    fs::write(home.join("java_pid2.hprof"), vec![0_u8; 200]).expect("test setup");
    // Sizing the whole home directory instead would sweep this in too.
    fs::write(home.join("notes.txt"), vec![0_u8; 9_000]).expect("test setup");

    let mut row = candidate("JVM heap dumps", home);
    row.kind = CleanupKind::HeapDumps;
    let mut dialog = dialog_with(vec![row]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert!(measured.exists);
    assert_eq!(measured.size.map(FileSize::bytes), Some(300));
    assert!(
        measured.checked,
        "a heap dump is never wanted, so finding one pre-ticks the row"
    );
}

#[test]
fn measure_is_reported_against_the_active_panel_directory() {
    // `open` resolves relative candidates against the active panel, so the same
    // relative name measures differently from two different directories.
    let first = TempDir::new().expect("test setup");
    let second = TempDir::new().expect("test setup");
    for (dir, bytes) in [(&first, 64_usize), (&second, 256_usize)] {
        let target: &Path = &dir.path().join("target");
        fs::create_dir(target).expect("test setup");
        fs::write(target.join("blob"), vec![0_u8; bytes]).expect("test setup");
    }

    let mut from_first = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut from_first, first.path());
    let mut from_second = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut from_second, second.path());

    assert_eq!(from_first.candidates[0].size.map(FileSize::bytes), Some(64));
    assert_eq!(
        from_second.candidates[0].size.map(FileSize::bytes),
        Some(256)
    );
}

/// On Linux, pip and Playwright keep their caches in the XDG cache folder
/// (`~/.cache`), which is where the rows must look — not in a Windows-style
/// `AppData` folder that no Linux tool creates.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_cache_rows_scan_the_xdg_cache_folder() {
    let dialog = DiskCleanupDialog::new();
    for folder in ["pip", "ms-playwright", "JetBrains"] {
        let row = dialog
            .candidates
            .iter()
            .find(|row| row.path.ends_with(folder))
            .unwrap_or_else(|| panic!("no row scans a {folder} folder"));
        assert!(
            !row.path
                .components()
                .any(|part| part.as_os_str() == "AppData"),
            "{} scans {}",
            row.label,
            row.path.display()
        );
    }
}
