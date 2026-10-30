//! Purpose: One pass over the typed built-ins: sort, mark, help, navigation and bookmarks.

use super::*;

#[test]
fn test_execute_command_branches() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    let bookmark_dir = temp_dir.path().join("bookmark");
    fs::create_dir(&subdir).expect("test setup");
    fs::create_dir(&bookmark_dir).expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");
    fs::write(subdir.join("nested.txt"), b"nested").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.active_panel = crate::models::PanelSide::Left;

    assert!(execute_command(&mut app, "").is_err());
    assert!(execute_command(&mut app, "cd").is_err());
    assert!(execute_command(&mut app, "cd file.txt").is_err());

    execute_command(&mut app, "cd subdir").expect("test setup");
    assert!(app.get_active_panel().path.ends_with("subdir"));
    assert!(app.status_text().contains("Opened"));

    assert_navigation_to_filesystem_root(&mut app, temp_dir.path());

    assert_sort_mark_and_help(&mut app);

    assert_bookmark_commands(&mut app, &bookmark_dir);

    Ok(())
}

fn assert_sort_mark_and_help(app: &mut App) {
    execute_command(app, "sort name").expect("test setup");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Name);
    execute_command(app, "sort ext").expect("test setup");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Extension);
    execute_command(app, "sort size").expect("test setup");
    assert_eq!(app.get_active_panel().sort_mode, SortMode::Size);
    assert!(execute_command(app, "sort bogus").is_err());

    assert!(execute_command(app, "mark").is_err());
    execute_command(app, "mark all").expect("test setup");
    assert_ne!(app.get_active_panel().marked.len(), 0);
    execute_command(app, "unmark").expect("test setup");
    assert_eq!(app.get_active_panel().marked.len(), 0);
    assert!(execute_command(app, "mark nope").is_err());

    execute_command(app, "help").expect("test setup");
    assert!(matches!(app.overlay, Overlay::Help));
}

fn assert_navigation_to_filesystem_root(app: &mut App, root: &Path) {
    let root_path = root.ancestors().last().unwrap_or(root);
    execute_command(app, &format!("cd {}", root_path.display())).expect("test setup");
    assert_eq!(
        app.get_active_panel()
            .path
            .canonicalize()
            .expect("test setup"),
        root_path.canonicalize().expect("test setup")
    );
}

fn assert_bookmark_commands(app: &mut App, bookmark_dir: &Path) {
    assert!(execute_command(app, "bookmark").is_err());
    assert!(execute_command(app, "bookmark set").is_err());
    assert!(execute_command(app, "bookmark set a").is_err());
    assert!(execute_command(app, "bookmark set 99").is_err());
    assert!(execute_command(app, "bookmark foo 1").is_err());

    execute_command(app, "bookmark set 0").expect("test setup");
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(0).expect("slot"))
        .is_some());
    assert!(execute_command(app, "bookmark go 1").is_err());

    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        bookmark_dir.to_path_buf(),
    );
    execute_command(app, "bookmark go 0").expect("test setup");
    assert_eq!(app.get_active_panel().path, bookmark_dir);
}
