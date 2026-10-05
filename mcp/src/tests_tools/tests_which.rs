//! Purpose: `which` finds executables on PATH, as a shell would.

use super::*;
#[cfg(unix)]
use crate::tools::system::which_all;
use crate::tools::system::WhichParams;

/// Something every machine running these tests has on PATH.
#[test]
fn which_finds_an_executable_that_is_there() {
    let name = if cfg!(windows) { "cmd" } else { "sh" };

    let json = tool_json(
        &FfmServer::which(Parameters(WhichParams {
            name: name.to_owned(),
        }))
        .expect("which"),
    );
    assert_eq!(json["found"], true, "{name} should be on PATH");
    assert!(json["path"].is_string());
    assert!(!json["all"].as_array().expect("all").is_empty());
}

#[test]
fn which_says_so_when_there_is_nothing_to_find() {
    let json = tool_json(
        &FfmServer::which(Parameters(WhichParams {
            name: "ffm-no-such-executable-anywhere".to_owned(),
        }))
        .expect("which"),
    );
    assert_eq!(json["found"], false);
    assert!(json["path"].is_null());
}

/// A plain file named like the command must not shadow the real one: `which`
/// answers "what would run", and a README on PATH does not run.
#[cfg(unix)]
#[test]
fn which_skips_a_non_executable_decoy_earlier_on_path() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let decoy_dir = dir.path().join("decoys");
    let real_dir = dir.path().join("bin");
    write_file_at(&decoy_dir.join("mytool"), "just a note, mode 644");
    let real = real_dir.join("mytool");
    write_file_at(&real, "#!/bin/sh\n");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o755)).expect("chmod");

    let path_list = std::env::join_paths([&decoy_dir, &real_dir]).expect("join paths");
    assert_eq!(which_all("mytool", &path_list), vec![real]);
}

/// A path is not a name: `which` answers about PATH, and taking a path here
/// would quietly answer a different question.
#[test]
fn which_refuses_anything_with_a_directory_in_it() {
    for name in ["", "/bin/sh", "dir\\thing"] {
        assert!(
            FfmServer::which(Parameters(WhichParams {
                name: name.to_owned()
            }))
            .is_err(),
            "{name:?} must be refused"
        );
    }
}
