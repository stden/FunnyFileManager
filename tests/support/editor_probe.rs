//! Exercise the real editor probe with isolated child-process environment.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use funny_file_manager::{
    editor::handle_edit,
    models::{App, Panel},
    status::Status,
    WritableTestBackend,
};
use ratatui::Terminal;
use tempfile::TempDir;

const CHILD_ROOT: &str = "FFM_TEST_EDITOR_PROBE_ROOT";
const ARGV_LOG: &str = "FFM_TEST_EDITOR_PROBE_ARGS";
const TEST_NAME: &str = "handle_edit_probes_for_the_code_cli_and_uses_it_when_present";
const TIMEOUT: Duration = Duration::from_secs(20);

pub fn verify() -> io::Result<()> {
    if let Some(directory) = std::env::var_os(CHILD_ROOT) {
        return exercise_probe(Path::new(&directory));
    }

    let directory = TempDir::new()?;
    let file = directory.path().join("notes.txt");
    fs::write(&file, "fixture")?;
    let bin = directory.path().join("bin");
    create_fake_code(&bin)?;

    let argv = directory.path().join("argv.txt");
    let output = directory.path().join("child-output.txt");
    let log = fs::File::create(&output)?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        // Only this child sees the fake CLI. No process-global environment is
        // changed while the integration binary runs its other tests in parallel.
        .env(CHILD_ROOT, directory.path())
        .env(ARGV_LOG, &argv)
        .env("PATH", &bin)
        // A failed probe must fail safely instead of opening a host editor.
        .env("EDITOR", directory.path().join("missing-editor"))
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    run_child(&mut command, &output)?;

    assert_eq!(
        fs::read_to_string(argv)?,
        format!("--version\n-w\n-n\n{}\n", file.display()),
        "the fake CLI must receive the version probe, launch flags and selected file"
    );
    Ok(())
}

fn create_fake_code(bin: &Path) -> io::Result<()> {
    fs::create_dir(bin)?;
    let fake_code = bin.join("code");
    fs::write(
        &fake_code,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$FFM_TEST_EDITOR_PROBE_ARGS\"\nexit 0\n",
    )?;
    fs::set_permissions(&fake_code, fs::Permissions::from_mode(0o755))
}

fn exercise_probe(directory: &Path) -> io::Result<()> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(directory.to_path_buf())?;
    app.right_panel = Panel::new(directory.to_path_buf())?;
    let selected = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "notes.txt")
        .expect("fixture file is listed");
    app.left_panel.select(Some(selected));
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24))?;
    let _restore = RestoreRawMode;
    // On a headless runner, resuming raw mode can fail after the launch. The
    // reported outcome and recorded arguments prove the real launch completed.
    let _ = handle_edit(&mut terminal, &mut app);
    assert_eq!(
        app.status,
        Status::Edited(directory.join("notes.txt").display().to_string())
    );
    Ok(())
}

struct RestoreRawMode;

impl Drop for RestoreRawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run_child(command: &mut Command, log: &Path) -> io::Result<()> {
    let mut child = ChildGuard(command.spawn()?);
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.0.try_wait()? {
            assert!(
                status.success(),
                "fake editor workflow failed: {}",
                fs::read_to_string(log)?
            );
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "fake editor workflow exceeded its deadline",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
