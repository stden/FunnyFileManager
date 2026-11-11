//! Purpose: Which paths an operation takes: the marks, else the cursor.

use super::*;

#[test]
fn test_get_source_paths_marked() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("a.txt"), b"a")?;
    fs::write(temp.path().join("b.txt"), b"b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 2);
    Ok(())
}

#[test]
fn test_get_source_paths_selected() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("x.txt"), b"x")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    // Select x.txt
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "x.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 1);
    Ok(())
}
