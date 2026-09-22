//! Program arguments and terminal handoff use the same production launch path.
use super::*;
use crate::{models::Panel, terminal::NoopTerminalOps, WritableTestBackend};
use tempfile::TempDir;

fn fixture() -> (TempDir, App, Terminal<WritableTestBackend>) {
    let root = TempDir::new().expect("program directory");
    std::fs::write(root.path().join("my program.exe"), b"fixture").expect("file");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "my program.exe");
    app.left_panel.select(index);
    let terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    (root, app, terminal)
}

#[test]
fn launch_receives_literal_path_and_panel_working_directory() {
    let (root, mut app, mut terminal) = fixture();
    launch_with(
        &mut terminal,
        &mut app,
        &mut NoopTerminalOps,
        |path, cwd| {
            assert_eq!(path, root.path().join("my program.exe"));
            assert_eq!(cwd, root.path());
            Err(io::Error::other("deliberate launch failure"))
        },
    )
    .expect("terminal restored");
    assert!(app.status_text().contains("deliberate launch failure"));
    terminal.draw(|_| {}).expect("terminal remains usable");
}

#[test]
fn native_program_names_are_not_shell_expressions() {
    let path = Path::new("my program $name & data.exe");
    let command = program_command(path);
    assert_eq!(command.get_program(), path.as_os_str());
    assert_eq!(command.get_args().count(), 0);
}

#[cfg(windows)]
#[test]
fn cmd_file_with_spaces_runs_in_the_panel_directory() {
    let (root, mut app, mut terminal) = fixture();
    let script = root.path().join("my script.cmd");
    std::fs::write(
        &script,
        "@echo off\r\necho launched>result.txt\r\nexit /b 7\r\n",
    )
    .expect("script");
    app.left_panel.load_entries().expect("reload");
    assert!(app.left_panel.select_named("my script.cmd"));
    assert!(
        app.left_panel
            .get_selected_entry()
            .expect("script entry")
            .executable
    );
    launch_with(
        &mut terminal,
        &mut app,
        &mut NoopTerminalOps,
        |path, cwd| program_command(path).current_dir(cwd).status(),
    )
    .expect("terminal restored");
    assert_eq!(
        std::fs::read_to_string(root.path().join("result.txt"))
            .expect("output")
            .trim(),
        "launched"
    );
    assert!(app.status_text().contains('7'), "{}", app.status_text());
}

#[cfg(windows)]
#[test]
fn windows_scripts_and_installers_have_explicit_runners() {
    for (file, runner, flags) in [
        (
            "script name.PS1",
            "powershell.exe",
            vec!["-NoProfile", "-File"],
        ),
        ("package name.MSI", "msiexec.exe", vec!["/i"]),
    ] {
        let command = program_command(Path::new(file));
        assert_eq!(command.get_program(), runner);
        let mut expected: Vec<&std::ffi::OsStr> = flags.iter().map(std::ffi::OsStr::new).collect();
        expected.push(std::ffi::OsStr::new(file));
        assert_eq!(command.get_args().collect::<Vec<_>>(), expected);
    }
}
