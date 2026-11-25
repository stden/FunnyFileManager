//! Purpose: Unit tests for FTP/FTPS quick-connect URL parsing and credential safety.

use super::*;

#[test]
fn parses_bare_hosts_urls_users_ports_ipv6_and_directories() {
    let bare = parse_ftp_target("ftp.example.test").expect("bare");
    assert_eq!(bare.protocol, Protocol::Ftp);
    assert_eq!(bare.host.as_str(), "ftp.example.test");
    assert_eq!(bare.port, Port::FTP);
    assert_eq!(bare.user, None);
    assert_eq!(bare.remote_dir, None);

    let full = parse_ftp_target("  ftps://alice@files.example.test:2121/pub/iso/  ").expect("full");
    assert_eq!(full.protocol, Protocol::Ftps);
    assert_eq!(full.user.as_ref().map(UserName::as_str), Some("alice"));
    assert_eq!(full.host.as_str(), "files.example.test");
    assert_eq!(full.port.get(), 2121);
    assert_eq!(
        full.remote_dir.as_ref().map(RemotePath::as_str),
        Some("/pub/iso")
    );

    let root_slash = parse_ftp_target("ftp://example.test/").expect("root slash");
    assert_eq!(root_slash.remote_dir, None);

    let v6_port = parse_ftp_target("ftp://[::1]:990/work").expect("v6 port");
    assert_eq!(v6_port.host.as_str(), "::1");
    assert_eq!(v6_port.port.get(), 990);
    assert_eq!(
        v6_port.remote_dir.as_ref().map(RemotePath::as_str),
        Some("/work")
    );

    let v6_plain = parse_ftp_target("ftps://[::1]").expect("v6");
    assert_eq!(v6_plain.host.as_str(), "::1");
    assert_eq!(v6_plain.port, Port::FTP);
}

#[test]
fn rejects_embedded_passwords_and_unsupported_schemes() {
    let embedded = parse_ftp_target("ftp://alice:secret@example.test").expect_err("password");
    assert_eq!(embedded.problem, Problem::EmbeddedPassword);
    assert!(embedded.to_string().contains("embedded password"));

    let http = parse_ftp_target("https://example.test").expect_err("scheme");
    assert_eq!(http.problem, Problem::UnsupportedScheme);
    assert!(http.to_string().contains("ftp://"));
}

#[test]
fn rejects_malformed_hosts_ports_users_and_control_characters() {
    for (input, expected) in [
        ("   ", Problem::Empty),
        ("ftp://ex\nample.test", Problem::ControlCharacter),
        ("ftp://[::1", Problem::Character('[')),
        ("ftp://[::1]abc", Problem::Character('a')),
        ("ftp://[::1]:abc", Problem::NotAPort),
        ("ftp://example.test:0", Problem::NotAPort),
        ("ftp://@example.test", Problem::Empty),
    ] {
        assert_eq!(
            parse_ftp_target(input).unwrap_err().problem,
            expected,
            "{input}"
        );
    }
}
