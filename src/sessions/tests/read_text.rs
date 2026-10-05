//! Purpose: Reading a text file — what is refused, and that a FIFO never
//! blocks the read.

use super::{fs, TempDir};
use crate::sessions::{read_text, ReadError};

#[test]
fn read_text_refuses_what_it_must_not_open_or_hold() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("f");
    fs::write(&file, "héllo").expect("write");
    assert_eq!(read_text(&file, 16).expect("text"), "héllo");
    assert!(matches!(
        read_text(&file, 3),
        Err(ReadError::TooLarge { limit: 3 })
    ));
    assert!(matches!(
        read_text(dir.path(), 16),
        Err(ReadError::NotAFile)
    ));
    fs::write(&file, [0xff, 0xfe]).expect("write");
    assert!(matches!(read_text(&file, 16), Err(ReadError::NotUtf8)));
    let missing = read_text(&dir.path().join("missing"), 16).expect_err("missing");
    assert!(
        matches!(&missing, ReadError::Io(error) if error.kind() == std::io::ErrorKind::NotFound)
    );
    let texts = [
        (ReadError::NotAFile, "not a regular file"),
        (ReadError::TooLarge { limit: 7 }, "larger than 7 bytes"),
        (ReadError::NotUtf8, "not UTF-8 text"),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
    assert!(!missing.to_string().is_empty());
}

/// A FIFO is never opened: opening one with no writer blocks for ever.
#[cfg(unix)]
#[test]
fn read_text_does_not_block_on_a_fifo() {
    let dir = TempDir::new().expect("temp dir");
    let fifo = dir.path().join("pipe");
    crate::test_support::make_fifo(&fifo).expect("mkfifo");
    let outcome =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            matches!(read_text(&fifo, 16), Err(ReadError::NotAFile))
        });
    assert_eq!(outcome, Some(true));
}
