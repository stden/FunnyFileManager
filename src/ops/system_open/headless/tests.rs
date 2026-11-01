//! Purpose: Association launchers are verified without opening desktop programs.

use super::*;

#[test]
fn association_command_keeps_the_selected_path_literal() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("two words' document.txt");
    let command = command(&path);
    let arguments: Vec<_> = command
        .get_args()
        .map(|argument| argument.to_string_lossy())
        .collect();
    #[cfg(windows)]
    assert!(arguments
        .last()
        .unwrap()
        .contains("two words'' document.txt"));
    #[cfg(not(windows))]
    assert_eq!(arguments.last().unwrap().as_ref(), path.to_string_lossy());
}

#[test]
fn association_result_reports_launcher_failure_without_running_it() {
    let root = tempfile::TempDir::new().unwrap();
    let path = root.path().join("document.txt");
    let error = open_with(&path, |_| {
        Ok(headless::ExternalOutcome {
            code: 9,
            output: Vec::new(),
        })
    })
    .unwrap_err();
    assert!(error.to_string().contains("code 9"));
    assert!(open_with(&path, |_| Ok(headless::ExternalOutcome {
        code: 0,
        output: Vec::new()
    }))
    .is_ok());
}
