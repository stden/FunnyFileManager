//! Purpose: Failed overwrite preparation must preserve the existing destination.

use super::*;
use crate::copy::{CopyPrompt, ExistingFiles};
use tempfile::TempDir;

fn overwrite(folder: &Path) -> CopyPrompt {
    let mut prompt = CopyPrompt::new("source", folder);
    prompt.existing = ExistingFiles::Overwrite;
    prompt
}

fn assert_original(folder: &Path, dest: &Path) {
    assert_eq!(fs::read(dest).unwrap(), b"original");
    assert_eq!(
        fs::read_dir(folder).unwrap().count(),
        1,
        "no staging leftovers"
    );
}

#[test]
fn a_source_removed_after_listing_cannot_destroy_the_destination() {
    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    let dest = target.path().join("data");
    fs::write(&dest, b"original").unwrap();

    let result = place_over_existing(
        &source.path().join("gone"),
        &dest,
        &overwrite(target.path()),
    );

    assert!(result.is_err());
    assert_original(target.path(), &dest);
}

#[cfg(unix)]
#[test]
fn a_refused_socket_copy_cannot_destroy_the_destination() {
    use crate::models::{FileName, Panel};

    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(source.path().join("data")).unwrap();
    let dest = target.path().join("data");
    fs::write(&dest, b"original").unwrap();
    let mut app = App::with_panels(
        Panel::new(source.path().to_path_buf()).unwrap(),
        Panel::new(target.path().to_path_buf()).unwrap(),
    );
    app.get_active_panel().marked.insert(FileName::new("data"));

    copy_with(&mut app, &overwrite(target.path()));

    assert!(app.status.to_string().contains("not a regular file"));
    assert_original(target.path(), &dest);
}

#[test]
fn failed_timestamp_copy_keeps_the_existing_destination() {
    use crate::utils::FORCE_TIMES_FAILURE;

    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    let file = source.path().join("data");
    fs::write(&file, b"replacement").unwrap();
    let dest = target.path().join("data");
    fs::write(&dest, b"original").unwrap();
    let mut prompt = overwrite(target.path());
    prompt.timestamps = true;

    FORCE_TIMES_FAILURE.set(true);
    let result = place_over_existing(&file, &dest, &prompt);
    FORCE_TIMES_FAILURE.set(false);

    assert!(result.is_err());
    assert_original(target.path(), &dest);
    assert_eq!(fs::read(file).unwrap(), b"replacement");
}

#[cfg(unix)]
#[test]
fn overwriting_another_hard_link_to_the_source_is_refused() {
    use crate::copy::Purpose;

    for purpose in [Purpose::Copy, Purpose::Move] {
        let root = TempDir::new().unwrap();
        let source = root.path().join("source");
        let dest = root.path().join("dest");
        fs::write(&source, b"original").unwrap();
        fs::hard_link(&source, &dest).unwrap();
        let mut prompt = overwrite(root.path());
        prompt.purpose = purpose;

        assert!(place_over_existing(&source, &dest, &prompt).is_err());

        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert_eq!(fs::read(&dest).unwrap(), b"original");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}
