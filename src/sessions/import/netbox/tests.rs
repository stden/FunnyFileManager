//! Unit tests: a `.netbox` export becomes sessions with every value checked;
//! a DTD, an entity, an oversized file or tree, or a wrong root is refused
//! outright; a bad entry is skipped with its reason; a saved password is
//! counted and never read. Every fixture is made up and written to a
//! temporary directory.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{decode, import, parse, MAX_FILE_BYTES, MAX_NODES, MAX_SESSIONS};
use crate::models::{
    Auth, Field, InvalidField, KeyFile, Origin, Problem, Protocol, RemotePath, Session, UserName,
};
use crate::sessions::import::{ImportError, Imported, SkipReason, Skipped};
use crate::sessions::ReadError;

/// `NetBox`'s export of one made-up session. The password is 100 made-up hex
/// digits: the importer counts it and never decodes it.
const EXAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<NetBox version="2.1">
  <Sessions>
    <Session name="demo@example.test">
      <HostName>example.test</HostName>
      <PortNumber>2222</PortNumber>
      <UserName>demo</UserName>
      <FSProtocol>SFTP%20(SCP)</FSProtocol>
      <RemoteDirectory>%2Fvar%2Fwww</RemoteDirectory>
      <Password>A35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C4D5E6F7A8B9CA35C</Password>
    </Session>
  </Sessions>
</NetBox>
"#;

const FILE: &str = "/srv/x/demo.netbox";

fn sessions(body: &str) -> String {
    format!("<NetBox><Sessions>{body}</Sessions></NetBox>")
}

fn parsed(xml: &str) -> Imported {
    parse(xml, Path::new(FILE)).expect("a NetBox export")
}

fn only_session(xml: &str) -> Session {
    let imported = parsed(xml);
    assert_eq!(imported.skipped, [], "nothing skipped");
    let [session] = imported.sessions.as_slice() else {
        panic!("one session: {:?}", imported.sessions);
    };
    session.clone()
}

fn only_skip(xml: &str) -> Skipped {
    let imported = parsed(xml);
    assert_eq!(imported.sessions, [], "nothing imported");
    let [skipped] = imported.skipped.as_slice() else {
        panic!("one skipped entry: {:?}", imported.skipped);
    };
    skipped.clone()
}

#[test]
fn the_example_export_becomes_one_session() {
    let imported = parsed(EXAMPLE);
    assert_eq!(imported.passwords_left_out, 1);
    let [session] = imported.sessions.as_slice() else {
        panic!("one session");
    };
    assert_eq!(session.name.as_str(), "demo@example.test");
    assert_eq!(session.host.as_str(), "example.test");
    assert_eq!(session.port.get(), 2222);
    assert_eq!(session.user.as_ref().map(UserName::as_str), Some("demo"));
    assert_eq!(session.protocol, Protocol::Sftp);
    assert_eq!(session.remote_dir, RemotePath::parse("/var/www").ok());
    assert_eq!(session.auth, Auth::Password);
    assert_eq!(session.origin, Origin::NetBoxFile(FILE.into()));
    assert!(
        !format!("{imported:?}").contains("A35C4D"),
        "the password is not carried"
    );
}

#[test]
fn import_reads_the_file_it_is_given() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("demo.netbox");
    fs::write(&file, EXAMPLE).expect("write");
    let imported = import(&file).expect("imported");
    assert_eq!(imported.sessions.len(), 1);
    assert_eq!(imported.sessions[0].origin, Origin::NetBoxFile(file));
}

#[test]
fn import_refuses_a_missing_or_oversized_file_before_parsing_it() {
    let dir = TempDir::new().expect("temp dir");
    let missing = import(&dir.path().join("none.netbox")).expect_err("missing");
    assert!(matches!(missing, ImportError::Read(ReadError::Io(_))));
    let big = dir.path().join("big.netbox");
    let padding = " ".repeat(usize::try_from(MAX_FILE_BYTES).expect("fits"));
    fs::write(&big, format!("<NetBox>{padding}</NetBox>")).expect("write");
    assert!(matches!(
        import(&big),
        Err(ImportError::Read(ReadError::TooLarge {
            limit: MAX_FILE_BYTES
        }))
    ));
}

#[test]
fn a_session_without_a_name_is_named_netbox_s_way() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><UserName>u</UserName><PortNumber>2200</PortNumber></Session>",
    ));
    assert_eq!(session.name.as_str(), "u@h.test:2200");
    let bare = only_session(&sessions("<Session><HostName>h.test</HostName></Session>"));
    assert_eq!(bare.name.as_str(), "h.test");
    assert_eq!(bare.port.get(), 22, "SFTP's port when none is given");
    assert_eq!(bare.user, None);
}

#[test]
fn blank_optional_settings_are_not_set() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><UserName> </UserName><RemoteDirectory/><PublicKeyFile></PublicKeyFile></Session>",
    ));
    assert_eq!(session.user, None);
    assert_eq!(session.remote_dir, None);
    assert_eq!(session.auth, Auth::Password);
}

#[test]
fn a_public_key_file_logs_in_by_key() {
    let session = only_session(&sessions(
        "<Session><HostName>h.test</HostName><PublicKeyFile>%2Fhome%2Fbob%2F.ssh%2Fid</PublicKeyFile></Session>",
    ));
    assert_eq!(
        session.auth,
        Auth::KeyFile(KeyFile::new("/home/bob/.ssh/id").expect("key"))
    );
}

#[test]
fn protocols_are_read_by_number_or_by_name() {
    let table = [
        ("0", "", Protocol::Scp),
        ("1", "", Protocol::Sftp),
        ("2", "", Protocol::Sftp),
        ("5", "", Protocol::Ftp),
        ("5", "0", Protocol::Ftp),
        ("5", "3", Protocol::Ftps),
        ("ftp", "1", Protocol::Ftps),
        ("scp", "1", Protocol::Scp),
        ("SFTP", "", Protocol::Sftp),
        (" sftp%20(scp) ", "", Protocol::Sftp),
        ("FTPS", "", Protocol::Ftps),
        ("", "", Protocol::Sftp),
    ];
    for (fs_protocol, ftps, expected) in table {
        let session = only_session(&sessions(&format!(
            "<Session><HostName>h.test</HostName><FSProtocol>{fs_protocol}</FSProtocol><Ftps>{ftps}</Ftps></Session>"
        )));
        assert_eq!(session.protocol, expected, "{fs_protocol:?} / {ftps:?}");
    }
    for unsupported in ["6", "7", "WebDAV", "S3"] {
        let skipped = only_skip(&sessions(&format!(
            "<Session name=\"w\"><HostName>h.test</HostName><FSProtocol>{unsupported}</FSProtocol></Session>"
        )));
        assert_eq!(
            skipped.reason,
            SkipReason::UnsupportedProtocol,
            "{unsupported}"
        );
    }
}

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

#[test]
fn passwords_are_counted_only_when_there_is_one() {
    let imported = parsed(&sessions(
        "<Session><HostName>a.test</HostName><Password>00</Password></Session>\
         <Session><HostName>b.test</HostName><Password> </Password></Session>\
         <Session><HostName>c;</HostName><Password>11</Password></Session>",
    ));
    assert_eq!(
        imported.passwords_left_out, 2,
        "a skipped entry's counts too"
    );
    assert_eq!(imported.sessions.len(), 2);
}

#[test]
fn sessions_are_read_from_every_sessions_list_and_nothing_else() {
    let imported = parsed(
        "<NetBox><Sessions><Session><HostName>a.test</HostName></Session><Other/></Sessions>\
         <Sessions><Session><HostName>b.test</HostName></Session></Sessions>\
         <Session><HostName>c.test</HostName></Session></NetBox>",
    );
    let hosts: Vec<&str> = imported.sessions.iter().map(|s| s.host.as_str()).collect();
    assert_eq!(hosts, ["a.test", "b.test"]);
    assert_eq!(parsed("<NetBox/>"), Imported::default());
}

#[test]
fn a_document_that_is_not_a_netbox_export_is_refused() {
    let path = Path::new(FILE);
    assert!(matches!(
        parse("<WinSCP><Sessions/></WinSCP>", path),
        Err(ImportError::NotNetBox)
    ));
    assert!(matches!(parse("<NetBox>", path), Err(ImportError::Xml(_))));
    assert!(matches!(parse("", path), Err(ImportError::Xml(_))));
}

/// No DTD is accepted, so no entity is ever expanded or fetched: not an
/// internal one, not a billion laughs, not an external file.
#[test]
fn a_dtd_is_refused_so_no_entity_is_expanded_or_fetched() {
    let documents = [
        r#"<!DOCTYPE NetBox [<!ENTITY h "example.test">]><NetBox><Sessions><Session><HostName>&h;</HostName></Session></Sessions></NetBox>"#,
        r#"<!DOCTYPE NetBox [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;">]><NetBox>&c;</NetBox>"#,
        r#"<!DOCTYPE NetBox SYSTEM "file:///etc/hostname"><NetBox/>"#,
        r#"<!DOCTYPE NetBox [<!ENTITY x SYSTEM "file:///etc/hostname">]><NetBox>&x;</NetBox>"#,
    ];
    for document in documents {
        let outcome = parse(document, Path::new(FILE));
        assert!(
            matches!(
                &outcome,
                Err(ImportError::Xml(roxmltree::Error::DtdDetected))
            ),
            "{document}: {outcome:?}"
        );
    }
    assert!(matches!(
        parse("<NetBox>&undeclared;</NetBox>", Path::new(FILE)),
        Err(ImportError::Xml(_))
    ));
}

#[test]
fn a_tree_past_the_node_limit_is_refused() {
    let nodes = usize::try_from(MAX_NODES).expect("fits");
    let document = format!("<NetBox>{}</NetBox>", "<a/>".repeat(nodes));
    assert!(document.len() < usize::try_from(MAX_FILE_BYTES).expect("fits"));
    assert!(matches!(
        parse(&document, Path::new(FILE)),
        Err(ImportError::Xml(roxmltree::Error::NodesLimitReached))
    ));
}

#[test]
fn more_sessions_than_one_import_takes_are_refused_and_the_limit_is_not() {
    let entry = "<Session><HostName>h.test</HostName></Session>";
    let at_limit = sessions(&entry.repeat(MAX_SESSIONS));
    let imported = parse(&at_limit, Path::new(FILE)).expect("at the limit");
    assert_eq!(imported.sessions.len(), 1, "the same name each time");
    assert_eq!(imported.skipped.len(), MAX_SESSIONS - 1);
    let over = sessions(&entry.repeat(MAX_SESSIONS + 1));
    assert!(matches!(
        parse(&over, Path::new(FILE)),
        Err(ImportError::TooManySessions {
            limit: MAX_SESSIONS
        })
    ));
}

#[test]
fn decoding_takes_whole_escapes_of_utf8_and_nothing_else() {
    assert_eq!(decode("SFTP%20(SCP)", "x"), Ok("SFTP (SCP)".to_owned()));
    assert_eq!(decode("%2f%2F", "x"), Ok("//".to_owned()));
    assert_eq!(decode("%E2%82%AC+1", "x"), Ok("€+1".to_owned()));
    assert_eq!(decode("plain", "x"), Ok("plain".to_owned()));
    assert_eq!(decode("", "x"), Ok(String::new()));
    for bad in ["%", "%4", "a%", "%G1", "%1G", "%+1", "%ff", "%C3"] {
        assert_eq!(
            decode(bad, "x"),
            Err(SkipReason::BadEncoding("x")),
            "{bad:?}"
        );
    }
}
