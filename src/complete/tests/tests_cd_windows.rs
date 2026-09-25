//! Purpose: `cd` completion on a real Windows drive: a backslash token lists its directory, a typed forward slash is kept, a bare drive gets its root.

use super::*;

/// The temp directory as the user would type it: absolute, backslashed.
fn typed(tmp: &TempDir) -> String {
    tmp.path().display().to_string()
}

#[test]
fn cd_lists_a_directory_after_its_trailing_backslash() {
    // The user's report: `cd C:\work\` offered nothing, because the backslash
    // was matched as part of a leaf name and no name held one.
    let tmp = cwd_with(&["alpha/", "beta/"]);
    let root = typed(&tmp);
    let c = complete(&format!("cd {root}\\"), tmp.path()).expect("both directories");
    let mut listed = texts(&c);
    listed.sort();
    assert_eq!(
        listed,
        vec![format!("{root}\\alpha\\"), format!("{root}\\beta\\")]
    );
    assert_eq!(c.line, format!("cd {root}\\"), "ambiguous: the line stays");
}

#[test]
fn cd_completes_a_leaf_typed_after_a_backslash() {
    let tmp = cwd_with(&["alpha/", "notes.txt"]);
    let root = typed(&tmp);
    let c = complete(&format!("cd {root}\\al"), tmp.path()).expect("alpha");
    assert_eq!(c.line, format!("cd {root}\\alpha\\"));
    let c = complete(&format!("cd {root}\\no"), tmp.path()).expect("notes");
    assert_eq!(c.line, format!("cd {root}\\notes.txt "));
}

#[test]
fn cd_keeps_a_typed_forward_slash() {
    let tmp = cwd_with(&["alpha/"]);
    let root = typed(&tmp).replace('\\', "/");
    let c = complete(&format!("cd {root}/al"), tmp.path()).expect("alpha");
    assert_eq!(c.line, format!("cd {root}/alpha/"));
}

#[test]
fn cd_completes_a_bare_drive_to_its_root() {
    // `C:` is not absolute to `std::path`, so it would be joined onto the
    // panel directory and complete nothing. The drive the temp dir is on
    // certainly exists.
    let tmp = empty_cwd();
    let Some(std::path::Component::Prefix(prefix)) = tmp.path().components().next() else {
        panic!(
            "a Windows temp dir has a drive prefix: {}",
            tmp.path().display()
        );
    };
    let drive = prefix.as_os_str().to_string_lossy().into_owned();
    let c = complete(&format!("cd {drive}"), tmp.path()).expect("the drive root");
    assert_eq!(c.line, format!("cd {drive}\\"));
}
