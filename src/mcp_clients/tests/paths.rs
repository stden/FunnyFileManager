//! Purpose: The server and folder paths a registration writes: Unicode, absolute,
//! in their ordinary spelling, and a binary only for a found server.

use super::*;

#[test]
fn a_server_path_must_be_unicode_and_a_folder_must_be_absolute() {
    let binary = absolute("/srv/x/bin/ffm-mcp");
    assert_eq!(server().as_str(), binary.to_str().unwrap());
    assert_eq!(server().as_path(), binary);
    assert_eq!(folder().as_path(), absolute("/srv/x/project"));
    assert_eq!(ProjectDir::new(PathBuf::from("relative/dir")), None);
    assert_eq!(ProjectDir::new(PathBuf::from("s3://bucket")), None);
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_unicode_is_handed_back() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let raw = PathBuf::from(OsString::from_vec(b"/srv/x/\xff/ffm-mcp".to_vec()));
    assert_eq!(ServerBinary::new(raw.clone()), Err(raw.clone()));
    assert_eq!(ProjectDir::new(raw), None);
}

/// Review finding: the panel's canonical path on Windows is `\\?\C:\…`,
/// which went into `FFM_MCP_ROOT` as it was.
#[cfg(windows)]
#[test]
fn a_project_folder_is_kept_in_its_ordinary_spelling() {
    let folder = ProjectDir::new(PathBuf::from(r"\\?\C:\srv\project")).unwrap();
    assert_eq!(folder.as_str(), r"C:\srv\project");
    let share = ProjectDir::new(PathBuf::from(r"\\?\UNC\host\share\p")).unwrap();
    assert_eq!(share.as_str(), r"\\host\share\p");
}

#[test]
fn only_a_found_server_has_a_binary() {
    assert_eq!(ServerState::Found(server()).binary(), Some(&server()));
    assert_eq!(ServerState::Missing.binary(), None);
    assert_eq!(ServerState::NotUnicode(PathBuf::from("/x")).binary(), None);
}
