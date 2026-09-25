//! Unit tests: what an import reports reads the way it should, and a name
//! from the file is made safe to print before it is reported.

use super::{ImportError, ImportReport, SkipReason, Skipped};
use crate::models::{Field, InvalidField, Problem};
use crate::sessions::ReadError;

#[test]
fn a_skipped_name_is_made_printable_and_short() {
    let skipped = Skipped::new("evil\u{1b}[2J\u{7}", SkipReason::Duplicate);
    assert_eq!(skipped.name(), "evil\u{FFFD}[2J\u{FFFD}");
    let long = Skipped::new(&"n".repeat(200), SkipReason::Duplicate);
    assert_eq!(long.name().chars().count(), 80);
    assert_eq!(
        Skipped::new("web", SkipReason::Duplicate).to_string(),
        "web: an earlier entry has the same name"
    );
}

#[test]
fn every_skip_reason_says_why() {
    let reasons = [
        (
            SkipReason::Invalid(InvalidField::new(Field::Port, Problem::NotAPort)),
            "Port: must be a number from 1 to 65535",
        ),
        (
            SkipReason::BadEncoding("HostName"),
            "HostName: not valid URL-encoded text",
        ),
        (
            SkipReason::Repeated("UserName"),
            "UserName is given more than once",
        ),
        (
            SkipReason::UnsupportedProtocol,
            "a protocol other than SFTP, SCP, FTP or FTPS",
        ),
        (SkipReason::Duplicate, "an earlier entry has the same name"),
        (
            SkipReason::ManualEntryKept,
            "already stored and changed by hand; kept",
        ),
    ];
    for (reason, text) in reasons {
        assert_eq!(reason.to_string(), text);
    }
    assert_eq!(
        SkipReason::from(InvalidField::new(Field::Host, Problem::Empty)),
        SkipReason::Invalid(InvalidField::new(Field::Host, Problem::Empty))
    );
}

#[test]
fn the_report_and_the_errors_read_as_one_line_each() {
    let report = ImportReport {
        imported: 3,
        skipped: vec![Skipped::new("x", SkipReason::Duplicate)],
        passwords_left_out: 0,
    };
    assert_eq!(report.to_string(), "3 imported, 1 skipped");
    assert_eq!(
        ImportError::Read(ReadError::NotAFile).to_string(),
        "cannot read the file: not a regular file"
    );
    assert_eq!(
        ImportError::NotNetBox.to_string(),
        "not a NetBox session export"
    );
    assert_eq!(
        ImportError::TooManySessions { limit: 9 }.to_string(),
        "more than 9 sessions in one file"
    );
    assert_eq!(
        ImportError::TooDeep { limit: 32 }.to_string(),
        "elements nested more than 32 deep"
    );
    let xml = roxmltree::Document::parse("<a>").expect_err("unclosed");
    assert!(ImportError::Xml(xml)
        .to_string()
        .starts_with("not a usable XML file: "),);
}
