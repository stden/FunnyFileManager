//! Purpose: unit tests for the port and the default session name it shapes.

use super::*;

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
