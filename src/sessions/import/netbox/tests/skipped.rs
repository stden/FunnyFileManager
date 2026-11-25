//! Purpose: An entry with a bad, repeated or badly encoded value, or a name
//! already taken, is skipped with its reason.

use super::{only_skip, parsed, sessions};
use crate::models::{Field, InvalidField, Problem};
use crate::sessions::import::{SkipReason, Skipped};

fn skip(body: &str) -> SkipReason {
    only_skip(&sessions(body)).reason
}

fn invalid(field: Field, problem: Problem) -> SkipReason {
    SkipReason::Invalid(InvalidField::new(field, problem))
}

#[test]
fn a_bad_address_skips_its_entry_with_the_field_and_the_reason() {
    assert_eq!(
        skip("<Session><HostName>a;b</HostName></Session>"),
        invalid(Field::Host, Problem::Character(';'))
    );
    assert_eq!(
        skip("<Session><UserName>u</UserName></Session>"),
        invalid(Field::Host, Problem::Empty)
    );
    assert_eq!(
        skip("<Session><HostName>h.test</HostName><PortNumber>0</PortNumber></Session>"),
        invalid(Field::Port, Problem::NotAPort)
    );
    assert_eq!(
        skip("<Session><HostName>h&#10;x</HostName></Session>"),
        invalid(Field::Host, Problem::ControlCharacter),
        "a character reference is decoded, then checked"
    );
}

#[test]
fn a_bad_login_or_place_skips_its_entry_with_the_field_and_the_reason() {
    assert_eq!(
        skip("<Session><HostName>h.test</HostName><UserName>-oX</UserName></Session>"),
        invalid(Field::User, Problem::LeadingDash)
    );
    assert_eq!(
        skip(
            "<Session><HostName>h.test</HostName><RemoteDirectory>www</RemoteDirectory></Session>"
        ),
        invalid(Field::RemoteDir, Problem::NotAbsolute)
    );
    assert_eq!(
        skip("<Session name=\"a%0Ab\"><HostName>h.test</HostName></Session>"),
        invalid(Field::Name, Problem::ControlCharacter)
    );
}

#[test]
fn a_setting_given_twice_or_badly_encoded_skips_its_entry() {
    assert_eq!(
        skip("<Session><HostName>a.test</HostName><HostName>b.test</HostName></Session>"),
        SkipReason::Repeated("HostName")
    );
    assert_eq!(
        skip("<Session><HostName>h%zz</HostName></Session>"),
        SkipReason::BadEncoding("HostName")
    );
    assert_eq!(
        skip("<Session name=\"%\"><HostName>h.test</HostName></Session>"),
        SkipReason::BadEncoding("name")
    );
}

#[test]
fn a_skipped_entry_is_reported_by_its_name_else_its_host() {
    let named = only_skip(&sessions(
        "<Session name=\"web\"><HostName>a b</HostName></Session>",
    ));
    assert_eq!(named.name(), "web");
    let by_host = only_skip(&sessions("<Session><HostName>a b</HostName></Session>"));
    assert_eq!(by_host.name(), "a b");
    let nameless = only_skip(&sessions("<Session/>"));
    assert_eq!(nameless.name(), "(unnamed session)");
}

#[test]
fn a_second_entry_with_the_same_name_is_skipped() {
    let imported = parsed(&sessions(
        "<Session name=\"web\"><HostName>a.test</HostName></Session>\
         <Session name=\"web\"><HostName>b.test</HostName></Session>",
    ));
    assert_eq!(imported.sessions.len(), 1);
    assert_eq!(imported.sessions[0].host.as_str(), "a.test");
    assert_eq!(
        imported.skipped,
        [Skipped::new("web", SkipReason::Duplicate)]
    );
}
