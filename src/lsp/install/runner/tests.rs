//! Purpose: Verify installer command isolation, failures, bounded waits, and cancellation.

use super::*;
use tempfile::TempDir;

#[test]
fn installers_receive_literal_arguments_and_the_managed_directory() {
    let root = TempDir::new().unwrap();
    let mut command = Command::new("python");
    command
        .args([
            "-c",
            "import os,sys; print(os.getcwd()); print(sys.argv[1])",
            "a & b",
        ])
        .current_dir(root.path());
    let output = run(&mut command, &AtomicBool::new(false)).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("a & b"));
    assert!(text.contains(&*root.path().to_string_lossy()));
}

#[test]
fn cancellation_and_deadlines_terminate_a_silent_installer() {
    let root = TempDir::new().unwrap();
    let mut command = Command::new("python");
    command
        .args(["-c", "import time; time.sleep(30)"])
        .current_dir(root.path());
    let started = Instant::now();
    assert_eq!(
        run_until(&mut command, &AtomicBool::new(false), started).unwrap_err(),
        InstallError::Timeout
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        run(&mut command, &AtomicBool::new(true)).unwrap_err(),
        InstallError::Cancelled
    );
}

#[test]
fn exit_failures_and_missing_programs_are_errors() {
    let root = TempDir::new().unwrap();
    let mut command = Command::new("python");
    command
        .args(["-c", "raise SystemExit(23)"])
        .current_dir(root.path());
    assert_eq!(
        run(&mut command, &AtomicBool::new(false)).unwrap_err(),
        InstallError::Command
    );
    assert!(matches!(
        run(
            &mut Command::new(root.path().join("absent")),
            &AtomicBool::new(false)
        ),
        Err(InstallError::Io(_))
    ));
}

#[test]
fn excessive_installer_output_is_bounded_and_terminates_the_command() {
    let root = TempDir::new().unwrap();
    let mut command = Command::new("python");
    command
        .args([
            "-c",
            "import sys,time; sys.stdout.write('x'*2000000); sys.stdout.flush(); time.sleep(30)",
        ])
        .current_dir(root.path());
    assert_eq!(
        run(&mut command, &AtomicBool::new(false)),
        Err(InstallError::Command)
    );
}
