//! Logging outcomes and payload protection tested through their owning module.

use super::*;
use std::{fs, io};
use tempfile::TempDir;

mod lifecycle;
mod security;

fn run_with_logs<F>(
    main_runner: F,
    action_log: &Path,
    error_log: &Path,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    run_with_logger(
        main_runner,
        |line| log_action_to(action_log, line),
        |line| log_error_to(error_log, line),
    )
}

#[test]
fn test_run_with_logs_records_error_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    let result = run_with_logs(|| Err(io::Error::other("boom").into()), &actions, &errors);

    assert!(result.is_err());
    assert!(fs::read_to_string(actions)?.contains("Started"));
    assert!(fs::read_to_string(errors)?.contains("Main error: boom"));
    Ok(())
}

#[test]
fn test_run_with_logs_records_success_at_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions.log");
    let errors = temp.path().join("errors.log");

    run_with_logs(|| Ok(()), &actions, &errors).expect("successful runner");

    let contents = fs::read_to_string(actions)?;
    assert!(contents.contains("Started"));
    assert!(contents.contains("Ended"));
    assert!(!errors.exists());
    Ok(())
}

#[test]
fn test_log_writers_ignore_unwritable_explicit_paths() -> io::Result<()> {
    let temp = TempDir::new()?;
    let actions = temp.path().join("actions");
    let errors = temp.path().join("errors");
    fs::create_dir(&actions)?;
    fs::create_dir(&errors)?;

    log_action_to(&actions, "action");
    log_error_to(&errors, "error");

    assert!(actions.is_dir());
    assert!(errors.is_dir());
    Ok(())
}
