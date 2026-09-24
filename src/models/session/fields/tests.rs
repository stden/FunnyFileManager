//! Unit tests: every checked session value takes what it should, refuses
//! what it must — with the field and the reason, never the value — and
//! `serde` refuses the same.

use super::{
    Field, HostName, InvalidField, KeyFile, Port, Problem, SessionName, UserName, MAX_HOST_CHARS,
    MAX_KEY_FILE_CHARS, MAX_NAME_CHARS, MAX_USER_CHARS,
};

/// A value's constructor with the value dropped: only the verdict counts.
type Check = fn(&str) -> Result<(), InvalidField>;

fn invalid(field: Field, problem: Problem) -> InvalidField {
    InvalidField::new(field, problem)
}

#[test]
fn text_is_trimmed_and_must_not_be_blank() {
    assert_eq!(
        SessionName::new("  demo  ").map(String::from),
        Ok("demo".to_owned())
    );
    assert_eq!(
        HostName::new(" example.test ").map(String::from),
        Ok("example.test".to_owned())
    );
    assert_eq!(
        SessionName::new(" \t "),
        Err(invalid(Field::Name, Problem::Empty))
    );
    assert_eq!(HostName::new(""), Err(invalid(Field::Host, Problem::Empty)));
    assert_eq!(
        UserName::new(" "),
        Err(invalid(Field::User, Problem::Empty))
    );
    assert_eq!(
        KeyFile::new(""),
        Err(invalid(Field::KeyFile, Problem::Empty))
    );
}

#[test]
fn control_characters_are_refused_everywhere() {
    let escape = "a\u{1b}[31mb";
    let control = Problem::ControlCharacter;
    assert_eq!(SessionName::new(escape), Err(invalid(Field::Name, control)));
    assert_eq!(UserName::new("bo\nb"), Err(invalid(Field::User, control)));
    assert_eq!(
        KeyFile::new("/k\u{7f}"),
        Err(invalid(Field::KeyFile, control))
    );
    assert_eq!(HostName::new("h\u{0}"), Err(invalid(Field::Host, control)));
}

#[test]
fn each_value_takes_its_limit_and_refuses_one_more() {
    let cases: [(Field, usize, Check); 4] = [
        (Field::Name, MAX_NAME_CHARS, |t| {
            SessionName::new(t).map(drop)
        }),
        (Field::Host, MAX_HOST_CHARS, |t| HostName::new(t).map(drop)),
        (Field::User, MAX_USER_CHARS, |t| UserName::new(t).map(drop)),
        (Field::KeyFile, MAX_KEY_FILE_CHARS, |t| {
            KeyFile::new(t).map(drop)
        }),
    ];
    for (field, limit, check) in cases {
        assert_eq!(check(&"a".repeat(limit)), Ok(()), "{field:?}");
        assert_eq!(
            check(&"a".repeat(limit + 1)),
            Err(invalid(field, Problem::TooLong { limit })),
            "{field:?}"
        );
    }
    // Characters, not bytes: a name of multi-byte letters is not cut short.
    assert!(UserName::new(&"я".repeat(MAX_USER_CHARS)).is_ok());
}

#[test]
fn a_host_is_a_dns_name_or_an_address_and_nothing_else() {
    for good in [
        "example.test",
        "10.0.0.1",
        "fe80::1",
        "db_1.example-host.test",
    ] {
        assert_eq!(HostName::new(good).map(String::from), Ok(good.to_owned()));
    }
    for (bad, ch) in [
        ("a b", ' '),
        ("a;b", ';'),
        ("a/b", '/'),
        ("héllo", 'é'),
        ("a@b", '@'),
    ] {
        assert_eq!(
            HostName::new(bad),
            Err(invalid(Field::Host, Problem::Character(ch))),
            "{bad:?}"
        );
    }
    assert_eq!(
        HostName::new("-oProxyCommand=x"),
        Err(invalid(Field::Host, Problem::Character('='))),
        "the `=` is caught before the dash"
    );
    assert_eq!(
        HostName::new("-host"),
        Err(invalid(Field::Host, Problem::LeadingDash))
    );
}

#[test]
fn a_user_is_any_printable_text_but_a_leading_dash() {
    assert_eq!(
        UserName::new("mail@example.test").map(String::from),
        Ok("mail@example.test".to_owned())
    );
    assert_eq!(
        UserName::new("First Last").map(|u| u.as_str().len()),
        Ok(10)
    );
    assert_eq!(
        UserName::new("-l"),
        Err(invalid(Field::User, Problem::LeadingDash))
    );
}

#[test]
fn a_port_is_a_whole_number_from_one_to_65535() {
    assert_eq!(Port::parse("22").map(Port::get), Ok(22));
    assert_eq!(Port::parse(" 65535 ").map(Port::get), Ok(65535));
    assert_eq!(Port::new(1).map(Port::get), Ok(1));
    for bad in ["0", "65536", "", "+22", "-1", "2 2", "ssh", "0x16"] {
        assert_eq!(
            Port::parse(bad),
            Err(invalid(Field::Port, Problem::NotAPort)),
            "{bad:?}"
        );
    }
    assert_eq!(Port::new(0), Err(invalid(Field::Port, Problem::NotAPort)));
    assert_eq!(Port::SSH.get(), 22);
    assert_eq!(Port::FTP.get(), 21);
    assert_eq!(Port::SSH.to_string(), "22");
}

#[test]
fn the_default_name_is_user_at_host_with_a_port_only_off_the_default() {
    let host = HostName::new("example.test").expect("host");
    let user = UserName::new("demo").expect("user");
    let port = |n| Port::new(n).expect("port");
    let name = |user, n| SessionName::default_for(user, &host, port(n), Port::SSH).to_string();
    assert_eq!(name(Some(&user), 22), "demo@example.test");
    assert_eq!(name(Some(&user), 2222), "demo@example.test:2222");
    assert_eq!(name(None, 22), "example.test");
    assert_eq!(name(None, 2121), "example.test:2121");
}

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
