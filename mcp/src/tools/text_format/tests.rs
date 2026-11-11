use super::*;

#[test]
fn utf16_byte_order_marks_and_old_mac_endings_are_distinguished() {
    assert_eq!(TextFormat::of(&[0xff, 0xfe, b'a', 0]).bom, "utf-16le");
    assert_eq!(TextFormat::of(&[0xfe, 0xff, 0, b'a']).bom, "utf-16be");
    let format = TextFormat::of(b"first\rsecond\r");
    assert_eq!(format.endings, "cr");
    assert_eq!(format.cr, 2);
    assert!(!format.final_newline);
}

#[test]
fn an_empty_file_has_no_last_byte_and_missing_files_keep_the_io_error() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("empty");
    fs::write(&path, []).unwrap();
    assert_eq!(last_byte(&path).unwrap(), None);
    fs::remove_file(&path).unwrap();
    assert_eq!(
        last_byte(&path).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
}
