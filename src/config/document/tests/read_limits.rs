//! Unit tests: the files a load refuses to read — not a regular file, too
//! large, not UTF-8 — each named in the error, and the 1 MiB limit itself.

use super::*;

#[test]
fn a_directory_or_an_oversized_file_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    fs::create_dir_all(&path).expect("a directory where the file goes");
    let error = read(&path).expect_err("a directory is not a settings file");
    assert_eq!(error.to_string(), "settings.yaml is not a regular file");

    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "");
    File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_len(MAX_FILE_BYTES + 1))
        .expect("grow the file");
    let error = read(&path).expect_err("too large");
    assert_eq!(error.to_string(), "settings.yaml exceeds 1 MiB");
}

/// A save blocked by the legacy file names it, or the user never learns that
/// `settings.json` is what stands in the way.
#[test]
fn a_legacy_file_that_cannot_be_read_is_named_and_not_replaced() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(legacy_path(&temp)).expect("a directory where the file goes");
    let error = read(&settings_path(&temp)).expect_err("not a file");
    assert_eq!(error.to_string(), "settings.json is not a regular file");
    let error = save(&settings_path(&temp), 1).expect_err("refused");
    assert_eq!(
        error.to_string(),
        "Not saved: settings.json is not a regular file"
    );
    assert!(!settings_path(&temp).exists());
}

#[test]
fn an_io_error_names_the_settings_file() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "");
    fs::write(&path, b"counter: \xff\n").expect("not UTF-8");
    let error = read(&path).expect_err("not UTF-8");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().starts_with("settings.yaml: "), "{error}");
    let error = save(&path, 1).expect_err("refused");
    assert!(
        error.to_string().starts_with("Not saved: settings.yaml: "),
        "{error}"
    );
}

/// Windows reports a path through a file as missing; Unix says why.
#[cfg(unix)]
#[test]
fn a_path_through_a_file_names_the_settings_file() {
    let temp = TempDir::new().expect("tempdir");
    let blocked = temp.path().join("file");
    fs::write(&blocked, "").expect("a file where a directory goes");
    let error = read(&blocked.join(FILE_NAME)).expect_err("not a directory");
    assert!(error.to_string().starts_with("settings.yaml: "), "{error}");
}

#[test]
fn a_file_of_exactly_1_mib_is_read() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    let comment = format!("#{}\n", "x".repeat(1_048_574));
    assert_eq!(comment.len(), 1_048_576);
    write_file(&path, &comment);
    assert!(read(&path).expect("at the limit").is_some());
}

#[test]
fn a_small_file_is_read_as_text() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("dictionary.json");
    fs::write(&path, "{}").expect("fixture");
    assert_eq!(read_small_file(&path, "Dictionary").expect("read"), "{}");
    let missing = read_small_file(&temp.path().join("missing"), "Dictionary").expect_err("missing");
    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert!(missing.to_string().starts_with("Dictionary: "), "{missing}");
}
