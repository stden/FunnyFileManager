//! Purpose: unit tests for how a refusal reads, and `serde` going through
//! the same checks.

use super::*;

const PROBLEM_TEXTS: [(Problem, &str); 8] = [
    (Problem::Empty, "must not be empty"),
    (
        Problem::TooLong { limit: 3 },
        "must be at most 3 characters",
    ),
    (
        Problem::ControlCharacter,
        "must not hold control characters",
    ),
    (Problem::Character('\u{202e}'), "must not hold '\\u{202e}'"),
    (Problem::LeadingDash, "must not start with '-'"),
    (Problem::NotAbsolute, "must start with '/'"),
    (Problem::DotSegment, "must not hold '.' or '..' parts"),
    (Problem::NotAPort, "must be a number from 1 to 65535"),
];

const FIELD_LABELS: [(Field, &str); 6] = [
    (Field::Name, "Name"),
    (Field::Host, "Host"),
    (Field::Port, "Port"),
    (Field::User, "User"),
    (Field::RemoteDir, "Directory"),
    (Field::KeyFile, "Key file"),
];

#[test]
fn a_refusal_names_the_field_and_the_reason_but_never_the_value() {
    let error = HostName::new("secret;value").expect_err("refused");
    let text = error.to_string();
    assert_eq!(text, "Host: must not hold ';'");
    assert!(!text.contains("secret"));
    for (problem, text) in PROBLEM_TEXTS {
        assert_eq!(problem.to_string(), text);
    }
    for (field, label) in FIELD_LABELS {
        assert_eq!(field.label(), label);
    }
}

#[test]
fn serde_writes_the_plain_value_and_reads_through_the_check() {
    let name = SessionName::new("demo").expect("name");
    assert_eq!(serde_json::to_string(&name).expect("json"), "\"demo\"");
    assert_eq!(serde_json::to_string(&Port::SSH).expect("json"), "22");
    assert!(serde_json::from_str::<HostName>("\"a b\"").is_err());
    assert!(serde_json::from_str::<Port>("0").is_err());
    assert!(serde_json::from_str::<UserName>("\"-x\"").is_err());
    assert!(serde_json::from_str::<KeyFile>("\"\"").is_err());
    assert_eq!(
        serde_json::from_str::<KeyFile>("\"/home/bob/.ssh/id\"")
            .expect("key file")
            .to_string(),
        "/home/bob/.ssh/id"
    );
    assert_eq!(
        serde_json::from_str::<SessionName>("\" x \"")
            .expect("name")
            .as_str(),
        "x"
    );
    assert_eq!(u16::from(Port::FTP), 21);
    assert_eq!(Port::try_from(8022_u16).map(Port::get), Ok(8022));
    assert_eq!(KeyFile::new(" /k ").map(String::from), Ok("/k".to_owned()));
}
