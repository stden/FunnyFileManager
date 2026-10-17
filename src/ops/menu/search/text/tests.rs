//! Purpose: The chunked text scan: matches across chunk boundaries, split
//! characters, case folding, binary files, the time limit and big files.

use std::io::{self, Read};
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::{contains_text, file_contains, fold, Scan, CHUNK};

mod decoding;

fn later() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

fn scan(text: &[u8], needle: &str) -> Scan {
    contains_text(text, &fold(needle), later()).expect("an in-memory read")
}

/// `pad` bytes of `a`, then `rest`: puts `rest` wherever a chunk ends.
fn padded(pad: usize, rest: &str) -> Vec<u8> {
    let mut bytes = vec![b'a'; pad];
    bytes.extend_from_slice(rest.as_bytes());
    bytes
}

#[test]
fn finds_the_needle_whatever_its_case() {
    assert_eq!(scan(b"alpha TODO beta", "todo"), Scan::Found);
    assert_eq!(scan("Привет, МИР".as_bytes(), "мир"), Scan::Found);
    assert_eq!(scan(b"nothing here", "todo"), Scan::Absent);
}

#[test]
fn a_needle_across_a_chunk_boundary_is_found() {
    assert_eq!(scan(&padded(CHUNK - 3, "needle"), "needle"), Scan::Found);
}

/// `ж` is two bytes: the first ends one chunk, the second starts the next.
#[test]
fn a_character_split_between_chunks_is_decoded_whole() {
    assert_eq!(scan(&padded(CHUNK - 1, "жук"), "ЖУК"), Scan::Found);
}

#[test]
fn an_invalid_byte_reads_as_a_replacement_mark() {
    let mut bytes = b"abc".to_vec();
    bytes.push(0xFF);
    bytes.extend_from_slice(b"needle");
    assert_eq!(scan(&bytes, "c\u{FFFD}needle"), Scan::Found);
}

#[test]
fn a_sequence_cut_off_by_the_end_of_the_file_reads_as_a_replacement_mark() {
    let mut bytes = b"tail".to_vec();
    bytes.push(0xD0);
    assert_eq!(scan(&bytes, "tail\u{FFFD}"), Scan::Found);
}

#[test]
fn a_binary_file_is_not_searched() {
    assert_eq!(scan(b"needle\0rest", "needle"), Scan::Absent);
}

#[test]
fn a_final_sigma_folds_like_any_other() {
    assert_eq!(fold("ΟΔΟΣ"), fold("οδος"));
    assert_eq!(scan("στην οδος".as_bytes(), "ΟΔΟΣ"), Scan::Found);
}

#[test]
fn a_scan_past_its_deadline_says_so() {
    let found = contains_text(&b"needle"[..], "needle", Instant::now()).expect("read");
    assert_eq!(found, Scan::OutOfTime);
}

/// A read error ends the scan with the error, which the file search then
/// counts as a file without the text.
#[test]
fn a_failing_read_is_an_error() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("broken disk"))
        }
    }
    assert!(contains_text(Broken, "needle", later()).is_err());
}

/// The reported case: 10.5 MB, the text in the first line, and the file was
/// skipped for its size without a word.
#[test]
fn a_file_over_ten_megabytes_is_searched() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("big-needle.txt");
    let mut text = String::from("needle at top\n");
    text.push_str(&"0000001 INFO nothing to see\n".repeat(400_000));
    std::fs::write(&path, &text).expect("write");
    assert!(text.len() > 10 * 1024 * 1024);

    assert_eq!(file_contains(&path, "needle", later()), Scan::Found);
    assert_eq!(
        file_contains(&path, &fold("0000001 info"), later()),
        Scan::Found
    );
    assert_eq!(file_contains(&path, "absent", later()), Scan::Absent);
}

#[test]
fn a_folder_or_a_missing_file_holds_no_text() {
    let dir = TempDir::new().expect("dir");
    assert_eq!(file_contains(dir.path(), "x", later()), Scan::Absent);
    let missing = dir.path().join("missing");
    assert_eq!(file_contains(&missing, "x", later()), Scan::Absent);
}
