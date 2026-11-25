//! Purpose: unit tests for the text values: trimming, control characters,
//! limits, and what a host and a user may hold.

use super::*;

/// A value's constructor with the value dropped: only the verdict counts.
type Check = fn(&str) -> Result<(), InvalidField>;

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
