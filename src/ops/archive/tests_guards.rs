//! Purpose: Archive guards that run before anything is unpacked or passed to a
//! tool: a setuid member is refused, an archive name that reads as an option is
//! given as a path, and an extraction answer with no prompt open does nothing.

use super::tool::safe_archive_arg;
use super::*;
use crate::models::Panel;
use tempfile::TempDir;

#[test]
fn a_plain_listing_is_accepted() {
    let details = "-rw-r--r-- root/root 1 2026-01-01 12:00 a.txt";

    assert!(validate_listing("a.txt", details).is_ok());
}

#[test]
fn a_member_with_setuid_bits_is_refused() {
    let details = "-rwsr-xr-x root/root 0 2026-01-01 12:00 bin/tool";

    let error = validate_listing("bin/tool", details).expect_err("a setuid member");

    assert!(error.to_string().contains("setuid"), "{error}");
}

#[cfg(unix)]
#[test]
fn an_archive_name_that_reads_as_an_option_or_a_drive_is_given_as_a_path() {
    assert_eq!(
        safe_archive_arg(Path::new("-x.tar")).as_ref(),
        Path::new("./-x.tar")
    );
    assert_eq!(
        safe_archive_arg(Path::new("host:x.tar")).as_ref(),
        Path::new("./host:x.tar")
    );
    assert_eq!(
        safe_archive_arg(Path::new("plain.tar")).as_ref(),
        Path::new("plain.tar")
    );
}

#[test]
fn an_extraction_answer_with_no_prompt_open_changes_nothing() {
    let root = TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).unwrap(),
        Panel::new(root.path().to_path_buf()).unwrap(),
    );

    run_extract(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
}
