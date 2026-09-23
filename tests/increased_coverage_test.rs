//! Purpose: Adds targeted tests that increase coverage for models, utils, and optional S3.
//!
use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, DriveEntry, Panel, PanelSide, SortMode};
#[cfg(feature = "s3")]
use funny_file_manager::s3::{list_aws_profiles, VirtualPath};
use std::fs;
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_sort_mode_labels_and_cycle() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");

    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_drive_entry_display_variants() {
    let local = DriveEntry::Local(PathBuf::from("/tmp"));
    let s3 = DriveEntry::S3Profile("default".to_string());

    assert_eq!(local.display(), PathBuf::from("/tmp").display().to_string());
    assert_eq!(s3.display(), "S3: default");
}

#[test]
fn test_app_panel_accessors_and_switching() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left.clone())?;
    app.right_panel = Panel::new(right.clone())?;

    assert_eq!(app.get_active_panel_ref().path, left);
    assert_eq!(app.get_inactive_panel_ref().path, right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    assert_eq!(app.get_active_panel_ref().path, right);

    app.swap_panels();
    assert_eq!(app.left_panel.path, right);
    assert_eq!(app.right_panel.path, left);

    Ok(())
}

#[test]
fn test_panel_selected_path_and_action_names() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let alpha_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "alpha.txt")
        .expect("alpha.txt entry");
    panel.select(Some(alpha_index));

    assert_eq!(panel.selected_path(), Some(temp.path().join("alpha.txt")));
    assert_eq!(panel.action_entry_names(), vec!["alpha.txt".to_string()]);

    panel.marked.insert(FileName::from("beta.txt"));
    assert_eq!(panel.action_entry_names(), vec!["beta.txt".to_string()]);

    Ok(())
}

#[test]
fn test_panel_selected_path_for_parent_entry_is_none() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut panel = Panel::new(subdir)?;
    let parent_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("parent entry");
    panel.select(Some(parent_index));

    assert_eq!(panel.selected_path(), None);
    assert!(panel.action_entry_names().is_empty());

    Ok(())
}

#[test]
fn test_panel_ensure_selection_and_clear_marks() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));

    panel.marked.insert(FileName::from("file.txt"));
    panel.clear_marks();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_find_matching_variants() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("alphabet.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    assert!(panel.find_first_matching("alp"));
    let first = panel
        .get_selected_entry()
        .expect("selected first match")
        .name
        .clone();
    assert!(first.starts_with("alp"));

    assert!(panel.find_next_matching("alp"));
    let second = panel
        .get_selected_entry()
        .expect("selected next match")
        .name
        .clone();
    assert!(second.starts_with("alp"));

    assert!(!panel.find_first_matching(""));
    assert!(!panel.find_next_matching("zzz"));

    Ok(())
}

#[test]
fn test_panel_toggle_mark_selected_ignores_parent_entry() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut panel = Panel::new(subdir)?;
    let parent_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("parent entry");
    panel.select(Some(parent_index));

    assert!(!panel.toggle_mark_selected());
    assert!(panel.marked.is_empty());

    Ok(())
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_and_display_variants() {
    assert_eq!(
        VirtualPath::parse("/tmp/local"),
        VirtualPath::Local(PathBuf::from("/tmp/local"))
    );

    assert_eq!(
        VirtualPath::parse("s3://bucket-name/prefix").display(),
        "s3://default@bucket-name/prefix"
    );
    assert_eq!(
        VirtualPath::parse("s3://prod@bucket-name/prefix").display(),
        "s3://prod@bucket-name/prefix"
    );
    assert_eq!(VirtualPath::parse("s3://").display(), "s3://default@");
    assert_eq!(VirtualPath::parse("s3://prod@").display(), "s3://prod@");
}

#[cfg(feature = "s3")]
#[test]
fn test_list_aws_profiles_always_contains_default() {
    let profiles = list_aws_profiles();
    assert!(!profiles.is_empty());
    assert_eq!(profiles[0].name, "default");
    assert!(profiles.iter().any(|profile| profile.name == "default"));
}

#[test]
fn test_app_get_drives_and_all_drives_are_non_empty() {
    let drives = App::get_drives();
    assert!(!drives.is_empty());

    let all = App::get_all_drives();
    assert!(!all.is_empty());
    assert!(matches!(all[0], DriveEntry::Local(_)));
}

#[test]
fn test_panel_enter_file_is_noop() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let original_path = panel.path.clone();
    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file entry");
    panel.select(Some(file_index));

    panel.enter()?;

    assert_eq!(panel.path, original_path);
    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("file.txt")
    );
    Ok(())
}

#[test]
fn test_panel_next_previous_with_none_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("one.txt"), b"1")?;
    fs::write(temp.path().join("two.txt"), b"2")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);

    panel.next();
    assert_eq!(panel.selected(), Some(0));

    panel.select(None);
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_load_entries_retains_selected_name_after_reload() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let beta_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "beta.txt")
        .expect("beta entry");
    panel.select(Some(beta_index));

    fs::write(temp.path().join("gamma.txt"), b"g")?;
    panel.load_entries()?;

    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("beta.txt")
    );
    Ok(())
}

#[test]
fn test_panel_cycle_sort_mode_changes_reload_order() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("dir"))?;
    // Name sort: a.txt, b.log; Extension sort: b.log, a.txt (.log < .txt)
    fs::write(temp.path().join("a.txt"), b"1")?;
    fs::write(temp.path().join("b.log"), b"12345")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.sort_mode = SortMode::Name;
    panel.load_entries()?;
    let names_before: Vec<String> = panel
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();

    panel.cycle_sort_mode();
    panel.load_entries()?;
    let names_after: Vec<String> = panel
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();

    assert_ne!(panel.sort_mode, SortMode::Name);
    assert_ne!(names_before, names_after);
    Ok(())
}

#[test]
fn test_app_active_panel_mut_accessors() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left.clone())?;
    app.right_panel = Panel::new(right.clone())?;

    app.get_active_panel().path = left.join("child");
    assert_eq!(app.left_panel.path, left.join("child"));

    app.get_inactive_panel().path = right.join("other");
    assert_eq!(app.right_panel.path, right.join("other"));

    Ok(())
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_bucket_without_prefix() {
    assert_eq!(
        VirtualPath::parse("s3://bucket-name").display(),
        "s3://default@bucket-name"
    );
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_bucket_with_empty_bucket() {
    assert_eq!(
        VirtualPath::parse("s3://prod@/prefix").display(),
        "s3://prod@"
    );
}

#[test]
fn test_panel_action_methods_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    assert_eq!(panel.selected_path(), None);
    assert_eq!(panel.action_entry_names(), Vec::<String>::new());
    assert!(!panel.find_first_matching("anything"));
    assert!(!panel.find_next_matching("anything"));
    assert!(!panel.toggle_mark_selected());

    panel.next();
    panel.previous();
    panel.ensure_selection();
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_toggle_mark_selected_marks_and_unmarks_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file entry");
    panel.select(Some(file_index));

    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    assert!(panel.toggle_mark_selected());
    assert!(!panel.marked.contains("file.txt"));

    Ok(())
}

#[test]
fn test_panel_find_next_matching_wraps_back_to_first_match() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("alpine.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let alpine_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "alpine.txt")
        .expect("alpine entry");
    panel.select(Some(alpine_index));

    assert!(panel.find_next_matching("alp"));
    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("alpha.txt")
    );

    Ok(())
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_bucket_without_prefix() {
    assert_eq!(
        VirtualPath::parse("s3://prod@bucket-name").display(),
        "s3://prod@bucket-name"
    );
}

#[cfg(feature = "s3")]
#[test]
fn test_app_get_all_drives_contains_default_s3_profile() {
    let all = App::get_all_drives();
    assert!(all.iter().any(|entry| matches!(
        entry,
        DriveEntry::S3Profile(name) if name == "default"
    )));
}
