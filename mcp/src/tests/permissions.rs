//! Purpose: `permissions` applies and validates Unix modes and the read-only flag.

use super::*;
use crate::tools::permissions::PermissionsParams;

// ═══════════════════════════════════════════════════════════════
// permissions
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

#[cfg(unix)]
#[test]
fn permissions_applies_an_octal_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");

    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("600".to_owned()),
            readonly: None,
        }))
        .expect("setting a mode succeeds");

    assert_eq!(mode_of(&file), 0o600);
    assert!(
        tool_json(&result)["mode"]
            .as_str()
            .expect("the payload reports a mode")
            .ends_with("600"),
        "the payload must report what is on disk"
    );
}

#[cfg(unix)]
#[test]
fn permissions_rejects_a_non_octal_mode_and_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    let error = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("rwx".to_owned()),
            readonly: None,
        }))
        .expect_err("'rwx' is not an octal mode");

    assert!(
        error.message.contains("invalid octal mode"),
        "got: {}",
        error.message
    );
    assert_eq!(mode_of(&file), 0o600, "a rejected mode must not be applied");
}

#[cfg(unix)]
#[test]
fn permissions_rejects_a_mode_above_the_octal_maximum() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    // 0o10000 parses fine but would silently truncate to 0 once handed to
    // `from_mode`, stripping every permission bit from the file.
    let error = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("10000".to_owned()),
            readonly: None,
        }))
        .expect_err("a mode wider than 7777 is out of range");

    assert!(
        error.message.contains("exceeds maximum 7777"),
        "got: {}",
        error.message
    );
    assert_eq!(mode_of(&file), 0o600, "a rejected mode must not be applied");
}

#[cfg(unix)]
#[test]
fn permissions_applies_mode_after_readonly_so_the_explicit_mode_wins() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("key");
    write_file_at(&file, "secret");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");

    // readonly=true alone would land on 0o444; running it *after* the mode
    // would leave 0o400. The caller asked for 0o600, so 0o600 is what must
    // survive.
    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "key".to_owned(),
            mode: Some("600".to_owned()),
            readonly: Some(true),
        }))
        .expect("both fields together succeed");

    assert_eq!(mode_of(&file), 0o600);
    assert_eq!(
        tool_json(&result)["readonly"],
        false,
        "the payload is re-read from disk, where owner write is back"
    );
}

#[cfg(unix)]
#[test]
fn permissions_readonly_clears_every_write_bit() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let file = root.join("shared");
    write_file_at(&file, "data");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o664)).expect("chmod");

    let result = server
        .permissions(Parameters(PermissionsParams {
            path: "shared".to_owned(),
            mode: None,
            readonly: Some(true),
        }))
        .expect("setting readonly succeeds");

    assert_eq!(mode_of(&file), 0o444, "group and world write must go too");
    assert_eq!(tool_json(&result)["readonly"], true);
}
