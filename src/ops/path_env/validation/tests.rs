use super::*;

#[test]
fn unknown_variables_are_never_treated_as_literal_paths() {
    let text = if cfg!(windows) {
        "%ROOT%/bin"
    } else {
        "${ROOT}/bin"
    };
    assert_eq!(
        expand(text, |_| Some("/opt".to_owned())),
        Some("/opt/bin".to_owned())
    );
    assert_eq!(expand(text, |_| None), None);
}

#[test]
fn home_expansion_keeps_spaces_unicode_and_literal_shell_syntax() {
    let temp = tempfile::TempDir::new().expect("home");
    let home = temp.path().to_string_lossy().into_owned();
    assert_eq!(
        expand("~/a b/界/`literal`", |name| (name == "HOME")
            .then(|| home.clone())),
        Some(format!("{home}/a b/界/`literal`"))
    );
    assert_eq!(expand("~/bin", |_| None), None);
    let unknown = if cfg!(windows) {
        "%FFM_MISSING_93287%"
    } else {
        "$FFM_MISSING_93287"
    };
    assert_eq!(validate(unknown), Validation::Unresolved);
}

#[cfg(unix)]
#[test]
fn unix_variables_support_braces_and_bare_names_without_executing_shell_text() {
    let resolve = |name: &str| match name {
        "ROOT_1" => Some("/opt".to_owned()),
        "DIR" => Some("tools".to_owned()),
        _ => None,
    };
    assert_eq!(
        expand("$ROOT_1/${DIR}/bin", resolve),
        Some("/opt/tools/bin".to_owned())
    );
    assert_eq!(
        expand("/tmp/$(literal)", resolve),
        Some("/tmp/$(literal)".to_owned())
    );
    assert_eq!(expand("$ABSENT/bin", resolve), None);
}
