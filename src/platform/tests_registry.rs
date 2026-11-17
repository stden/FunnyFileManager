//! Purpose: Tests for the Windows registry string wrappers, which read the
//! real registry and never write to it.

/// The machine's environment key, which every Windows has.
#[cfg(windows)]
const MACHINE_ENVIRONMENT: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

/// The registry is only read here: the machine's PATH names System32 on
/// every Windows, whether it is stored expandable or not.
#[cfg(windows)]
#[test]
fn the_machine_path_reads_as_a_string_naming_system32() {
    use super::{read_registry_string, RegistryHive};

    let path = read_registry_string(RegistryHive::LocalMachine, MACHINE_ENVIRONMENT, "Path")
        .expect("readable")
        .expect("a Path value");
    assert!(
        path.text.to_ascii_lowercase().contains("system32"),
        "{path:?}"
    );
    assert!(!path.text.ends_with('\0'), "{path:?}");
}

#[cfg(windows)]
#[test]
fn a_missing_key_or_value_reads_as_none() {
    use super::{read_registry_string, RegistryHive};

    let value = read_registry_string(
        RegistryHive::LocalMachine,
        MACHINE_ENVIRONMENT,
        "FFM-no-such-value",
    );
    assert_eq!(value.expect("readable"), None);
    let key = read_registry_string(
        RegistryHive::CurrentUser,
        r"Software\FFM-no-such-key-7d1f0c",
        "Path",
    );
    assert_eq!(key.expect("readable"), None);
}

/// A number is not a string: refused rather than shown as one.
#[cfg(windows)]
#[test]
fn a_value_that_is_not_a_string_is_refused() {
    use super::{read_registry_string, RegistryHive};

    let number = read_registry_string(
        RegistryHive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentMajorVersionNumber",
    );
    assert_eq!(
        number.expect_err("a REG_DWORD").kind(),
        std::io::ErrorKind::InvalidData
    );
}

/// An update whose decision fails writes nothing. The value is one no
/// Windows has, in the current user's environment key, which every user
/// has: the key is opened for writing, the value read as missing, and the
/// decision's error comes back with nothing changed.
#[cfg(windows)]
#[test]
fn an_update_that_decides_against_writing_leaves_the_key_alone() {
    use super::{read_registry_string, update_registry_string, RegistryHive};

    const NAME: &str = "FFM-test-never-written";
    let mut seen = None;
    let refused =
        update_registry_string(RegistryHive::CurrentUser, "Environment", NAME, |current| {
            seen = Some(current);
            Err(std::io::Error::other("decided against it"))
        });
    assert_eq!(
        refused.expect_err("refused").to_string(),
        "decided against it"
    );
    assert_eq!(seen, Some(None), "the missing value reads as None");
    let after = read_registry_string(RegistryHive::CurrentUser, "Environment", NAME);
    assert_eq!(after.expect("readable"), None, "nothing was written");
}

#[cfg(windows)]
struct TempRegistryKey(String);

#[cfg(windows)]
impl TempRegistryKey {
    fn unique(tag: &str) -> Self {
        Self(format!(
            r"Software\FunnyFileManager-Test-{}-{tag}",
            std::process::id()
        ))
    }
}

#[cfg(windows)]
impl Drop for TempRegistryKey {
    fn drop(&mut self) {
        drop(
            std::process::Command::new("reg.exe")
                .args(["delete", &format!(r"HKCU\{}", self.0), "/f"])
                .output(),
        );
    }
}

#[cfg(windows)]
#[test]
fn update_registry_string_writes_and_reads_back_plain_and_expandable_values() {
    use super::{
        read_registry_string, update_registry_string, RegistryHive, RegistryString,
        RegistryStringKind,
    };
    assert_eq!(RegistryHive::CurrentUser.label(), "HKEY_CURRENT_USER");
    assert_eq!(RegistryHive::LocalMachine.label(), "HKEY_LOCAL_MACHINE");

    let key = TempRegistryKey::unique("roundtrip");
    update_registry_string(RegistryHive::CurrentUser, &key.0, "Entry", |prev| {
        assert_eq!(prev, None);
        Ok(RegistryString {
            kind: RegistryStringKind::Plain,
            text: "alpha".to_owned(),
        })
    })
    .expect("write plain");
    update_registry_string(RegistryHive::CurrentUser, &key.0, "Entry", |prev| {
        assert!(prev.is_some());
        Ok(RegistryString {
            kind: RegistryStringKind::Expandable,
            text: r"%SystemRoot%\System32".to_owned(),
        })
    })
    .expect("write expandable");
    let read_back = read_registry_string(RegistryHive::CurrentUser, &key.0, "Entry")
        .expect("readable")
        .expect("present");
    assert_eq!(read_back.kind, RegistryStringKind::Expandable);
    assert_eq!(read_back.text, r"%SystemRoot%\System32");
}

#[cfg(windows)]
#[test]
fn overlong_subkey_component_reports_os_error_from_open() {
    use super::{read_registry_string, RegistryHive};
    assert!(read_registry_string(RegistryHive::CurrentUser, &"a".repeat(300), "Path").is_err());
}
