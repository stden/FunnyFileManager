//! Purpose: a symbolic link that points nowhere still takes its name — every
//! "already existing files" choice treats it as the entry it is.
//!
//! `Path::exists` follows the link, so a dangling one looked like a free name:
//! F6 renamed the file over it, and F5 failed with "File exists".

use super::*;

/// `from/a.txt`, and `to/a.txt -> /nonexistent/a`.
fn onto_a_dangling_link() -> (App, TempDir, TempDir) {
    let (mut app, from, to) = app_with(&[("a.txt", b"new")]);
    std::os::unix::fs::symlink("/nonexistent/a", to.path().join("a.txt")).expect("test setup");
    mark(&mut app, "a.txt");
    (app, from, to)
}

fn link_kept(to: &Path) -> bool {
    fs::read_link(to.join("a.txt")).is_ok_and(|target| target == Path::new("/nonexistent/a"))
}

#[test]
fn skip_leaves_a_dangling_link_alone() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, to) = onto_a_dangling_link();
        let mut dialog = settings(to.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Skip;

        copy_with(&mut app, &dialog);

        assert!(link_kept(to.path()), "{purpose:?}: {}", app.status);
        assert_eq!(fs::read(from.path().join("a.txt")).unwrap(), b"new");
        assert!(
            app.status.to_string().contains("1 skipped"),
            "{}",
            app.status
        );
    }
}

#[test]
fn ask_puts_the_file_beside_a_dangling_link() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, _from, to) = onto_a_dangling_link();
        let mut dialog = settings(to.path());
        dialog.purpose = purpose;

        copy_with(&mut app, &dialog);

        assert!(link_kept(to.path()), "{purpose:?}: {}", app.status);
        assert_eq!(fs::read(to.path().join("a_1.txt")).unwrap(), b"new");
    }
}
