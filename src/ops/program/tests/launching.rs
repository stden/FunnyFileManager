//! Purpose: Enter on a program: the path and folder it gets, runners, and the transcript.

use super::*;

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
    assert_eq!(
        app.status,
        Status::CannotRun {
            program: root.path().join("my program.exe"),
            error: "deliberate launch failure".to_owned(),
        }
    );
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
        ("build tool.PY", "python", Vec::new()),
        ("deploy script.SH", "sh", Vec::new()),
    ] {
        let command = program_command(Path::new(file));
        assert_eq!(command.get_program(), runner);
        let mut expected: Vec<&std::ffi::OsStr> = flags.iter().map(std::ffi::OsStr::new).collect();
        expected.push(std::ffi::OsStr::new(file));
        assert_eq!(command.get_args().collect::<Vec<_>>(), expected);
    }
}

#[cfg(not(windows))]
#[test]
fn unix_scripts_without_execute_bit_use_explicit_interpreters() {
    for (file, runner, flags) in [
        ("build tool.PY", "python3", Vec::new()),
        ("deploy script.SH", "sh", Vec::new()),
        ("win script.CMD", "cmd.exe", vec!["/c"]),
    ] {
        let command = program_command(Path::new(file));
        assert_eq!(command.get_program(), runner);
        let mut expected: Vec<&std::ffi::OsStr> = flags.iter().map(std::ffi::OsStr::new).collect();
        expected.push(std::ffi::OsStr::new(file));
        assert_eq!(command.get_args().collect::<Vec<_>>(), expected);
    }
}

#[test]
fn zz_probe_empty_selection_absolute_error() {
    let mut app = App::with_panels(
        Panel::from_entries(
            std::path::PathBuf::new(),
            vec![crate::models::DirEntry {
                name: String::new(),
                ..Default::default()
            }],
        ),
        Panel::from_entries(std::path::PathBuf::new(), Vec::new()),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let error = launch_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
        panic!("must not launch")
    })
    .expect_err("empty path cannot be made absolute");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

/// A program started with Enter is part of the session record like a `:`
/// command: Ctrl+O shows what ran, where, and how it ended.
#[test]
fn a_program_started_with_enter_is_recorded_in_the_transcript() {
    let (root, mut app, mut terminal) = fixture();
    launch_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
        Ok(ExitStatus::default())
    })
    .expect("terminal restored");
    let program = root.path().join("my program.exe");
    assert_eq!(
        transcript(&app),
        [
            format!("{}> {}", root.path().display(), program.display()),
            format!("{}: {}", program.display(), ExitStatus::default()),
        ]
    );
    assert_eq!(
        app.status,
        Status::ProgramExited {
            program,
            status: ExitStatus::default(),
        }
    );
}
