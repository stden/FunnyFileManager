//! Purpose: Regular files are zeroed, emptied and renamed before they go; folders file by file.

use super::*;

#[test]
fn a_file_is_zeroed_before_it_is_removed() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("secret.txt");
    fs::write(&file, b"top secret payload").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten.len(), 1, "{:?}", witness.seen());
    assert_eq!(overwritten[0].path, file);
    assert_eq!(overwritten[0].bytes.len(), b"top secret payload".len());
    assert!(overwritten[0].bytes.iter().all(|&byte| byte == 0));
    assert!(!file.exists(), "the file is gone afterwards");
}

#[test]
fn the_zero_pass_covers_files_longer_than_one_buffer() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("large.bin");
    let payload: Vec<u8> = (0..(ZERO_CHUNK * 3 + 7))
        .map(|i| u8::try_from(i % 251).expect("below 251") + 1)
        .collect();
    fs::write(&file, &payload).expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten[0].bytes.len(), payload.len());
    assert!(overwritten[0].bytes.iter().all(|&byte| byte == 0));
}

#[test]
fn an_empty_file_is_wiped_without_a_zero_to_write() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("empty");
    fs::write(&file, b"").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    assert_eq!(witness.at(Stage::Overwritten)[0].bytes, Vec::<u8>::new());
    assert!(!file.exists());
}

#[test]
fn the_last_name_is_a_temporary_one_and_the_file_is_empty_by_then() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("named.txt");
    fs::write(&file, b"something").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let renamed = witness.at(Stage::Renamed);
    assert_eq!(renamed.len(), 1, "{:?}", witness.seen());
    assert_ne!(
        renamed[0].path, file,
        "the real name is gone before removal"
    );
    assert_eq!(renamed[0].path.parent(), Some(dir.path()));
    assert!(
        renamed[0]
            .path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(TEMP_PREFIX)),
        "{:?}",
        renamed[0].path
    );
    assert!(renamed[0].bytes.is_empty(), "truncated before the rename");
}

#[test]
fn a_directory_is_wiped_file_by_file_and_then_removed() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().join("project");
    let nested = root.join("src").join("deep");
    fs::create_dir_all(&nested).expect("mkdir");
    fs::write(root.join("a.txt"), b"aaaa").expect("write");
    fs::write(nested.join("b.txt"), b"bbbbbb").expect("write");
    let witness = Witness::install();

    wipe_path(&root).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten.len(), 2, "{:?}", witness.seen());
    assert!(overwritten
        .iter()
        .all(|seen| seen.bytes.iter().all(|&byte| byte == 0)));
    // Two files and three directories each carried a temporary name last.
    assert_eq!(witness.at(Stage::Renamed).len(), 5, "{:?}", witness.seen());
    assert!(!root.exists());
    assert!(
        fs::read_dir(dir.path()).expect("read dir").next().is_none(),
        "no temporary names left behind"
    );
}

#[test]
fn a_readonly_file_is_zeroed_and_removed() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("readonly.txt");
    fs::write(&file, b"secret").expect("write");
    let mut perms = fs::metadata(&file).expect("meta").permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms).expect("readonly");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    assert_eq!(witness.at(Stage::Overwritten).len(), 1);
    assert!(!file.exists());
}

#[test]
fn a_failed_removal_restores_the_original_name() {
    let dir = TempDir::new().expect("temp dir");
    let folder = dir.path().join("keep_name");
    fs::create_dir(&folder).expect("mkdir");
    WITNESS.with(|witness| {
        *witness.borrow_mut() = Some(Box::new(|stage: Stage, path: &Path| {
            if stage == Stage::Renamed {
                drop(fs::write(path.join("late.txt"), b"appeared"));
            }
        }));
    });

    let result = wipe_path(&folder);
    WITNESS.with(|witness| *witness.borrow_mut() = None);

    assert!(result.is_err());
    assert!(folder.is_dir(), "original directory name restored");
    assert!(folder.join("late.txt").is_file());
}
