//! Purpose: Unit tests for registry error conversion, read-only handle write
//! rejection, and `query_with` retry/error branches.

use windows_sys::Win32::Foundation::ERROR_MORE_DATA;

use super::key::Answer;
use super::value::query_with;
use super::*;
use crate::platform::RegistryStringKind;

#[test]
fn registry_helpers_cover_overflow_errors_and_readonly_writes() {
    let err = check(u32::MAX).unwrap_err();
    assert!(err.to_string().contains("registry error"), "{err}");
    assert!(
        update_registry_string(RegistryHive::CurrentUser, &"a".repeat(300), "Path", |_| Ok(
            RegistryString {
                kind: RegistryStringKind::Plain,
                text: String::new(),
            }
        ))
        .is_err()
    );
    let readonly = Key::open(RegistryHive::CurrentUser, "Environment")
        .unwrap()
        .expect("HKCU\\Environment");
    let name = nul_terminated(OsStr::new("FFM-readonly-test")).unwrap();
    let val = RegistryString {
        kind: RegistryStringKind::Plain,
        text: "denied".to_owned(),
    };
    assert!(readonly.set_string(&name, &val).is_err());
}

#[test]
fn query_with_reports_win32_failure_and_unstable_growing_values() {
    let query_err = query_with(|_| {
        Ok(Answer {
            status: 5,
            kind: 1,
            bytes: 0,
        })
    })
    .unwrap_err();
    assert_eq!(query_err.raw_os_error(), Some(5));
    let changing = query_with(|units| {
        Ok(Answer {
            status: ERROR_MORE_DATA,
            kind: 1,
            bytes: u32::try_from((units.len() + 4) * 2).unwrap(),
        })
    })
    .unwrap_err();
    assert_eq!(changing.kind(), io::ErrorKind::Interrupted);
}
