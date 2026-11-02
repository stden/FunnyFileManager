use std::fs;
use tempfile::TempDir;

use super::*;

#[test]
fn display_path_strips_windows_prefixes() {
    assert_eq!(
        display_path(Path::new(r"\\?\UNC\server\share")),
        r"\\server\share"
    );
    assert_eq!(display_path(Path::new(r"\\?\C:\Windows")), r"C:\Windows");
    assert_eq!(drive(Path::new("/")), std::path::MAIN_SEPARATOR.to_string());
    assert_eq!(drive(Path::new("relative")), "");
}

#[test]
fn capture_and_select_strip_shell_metacharacters_from_descriptions() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("a.txt"), "a").expect("a");
    fs::write(dir.path().join("b.txt"), "b").expect("b");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    for entry in &mut app.left_panel.entries {
        if entry.name == "a.txt" {
            entry.description = Some("safe; rm -rf /\n$(id) `uname` | cat > out & %X%".to_owned());
        } else if entry.name == "b.txt" {
            entry.description = Some("next\r\x1b[31m\"q\" 's' <in (sub) \\slash".to_owned());
        }
    }
    let mut ctx = capture(&app, "a.txt");
    assert_eq!(ctx.left.description, "safe rm -rf /id uname  cat  out  X");
    select(&mut ctx, &app, "b.txt");
    assert_eq!(ctx.left.description, "next[31mq s in sub slash");
}
