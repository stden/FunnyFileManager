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

fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect()
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
}

/// Remembers every pause prompt; otherwise does nothing.
#[derive(Default)]
struct PauseOps {
    prompts: Vec<String>,
}

impl TerminalOps<WritableTestBackend> for PauseOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn enter_alternate(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn leave_alternate(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn enable_mouse(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn disable_mouse(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn clear(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn show_cursor(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn wait_for_key(
        &mut self,
        _: &mut Terminal<WritableTestBackend>,
        prompt: &str,
    ) -> io::Result<()> {
        self.prompts.push(prompt.to_owned());
        Ok(())
    }
}

/// What the program printed sits on the terminal's own screen; the panels
/// would cover it at once, so they wait for a key, as the output's last line.
#[test]
fn the_panels_come_back_only_after_a_key_once_the_program_has_run() {
    let (_root, mut app, mut terminal) = fixture();
    app.program_pause = ProgramPause::UntilKey;
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Ok(ExitStatus::default())
    })
    .expect("terminal restored");
    assert_eq!(
        ops.prompts,
        [format!(
            "[{}] Press any key to return to FFM",
            ExitStatus::default()
        )]
    );
}

/// A program that never started printed nothing worth pausing for; the
/// status bar has the reason.
#[test]
fn a_launch_failure_returns_to_the_panels_at_once() {
    let (_root, mut app, mut terminal) = fixture();
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Err(io::Error::other("exec format error"))
    })
    .expect("terminal restored");
    assert!(ops.prompts.is_empty());
    assert_eq!(
        transcript(&app).last().map(String::as_str),
        Some(app.status_text().as_str())
    );
}

/// Without a person at the terminal (every test's `App::new()`, a replay)
/// nothing could answer the pause, so the panels return at once. On Windows
/// the pause used to wait for a console key that CI never sends.
#[test]
fn without_a_pause_the_panels_return_as_soon_as_the_program_ends() {
    let (_root, mut app, mut terminal) = fixture();
    assert_eq!(app.program_pause, ProgramPause::None);
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Ok(ExitStatus::default())
    })
    .expect("terminal restored");
    assert!(ops.prompts.is_empty(), "{:?}", ops.prompts);
}
