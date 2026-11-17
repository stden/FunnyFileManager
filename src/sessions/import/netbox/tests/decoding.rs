//! Purpose: `WinSCP`'s URL encoding is undone whole, or not at all.

use super::super::decode;
use crate::sessions::import::SkipReason;

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
