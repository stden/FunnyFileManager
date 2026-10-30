//! Purpose: Archives refused before anything is written: a link, a parent path, an unknown kind.

use super::*;

/// A tar holding a symlink is refused before anything is written — the
/// destination stays empty.
#[cfg(unix)]
#[test]
fn extract_archive_refuses_a_tar_with_a_link() {
    let temp = TempDir::new().expect("temp");
    let payload = temp.path().join("payload");
    fs::create_dir_all(&payload).expect("payload");
    fs::write(payload.join("hello.txt"), "hello").expect("file");
    std::os::unix::fs::symlink("/etc/hostname", payload.join("link")).expect("symlink");
    let archive = temp.path().join("linked.tar");
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("payload")
        .current_dir(temp.path())
        .status()
        .expect("tar runs");
    assert!(status.success());
    let destination = temp.path().join("out");
    let error = extract_archive(&archive, &destination).expect_err("a link is refused");
    assert!(error.to_string().contains("linked.tar"), "{error}");
    assert!(!destination.exists(), "nothing was written");
}

/// A zip with a `..` member is refused before `unzip` runs.
#[test]
fn extract_archive_refuses_a_zip_with_a_parent_path() {
    if !has("unzip") || !has("python3") {
        return;
    }
    let temp = TempDir::new().expect("temp");
    let escaping = temp.path().join("escape.zip");
    let script = format!(
        "import zipfile\nwith zipfile.ZipFile({escaping:?}, 'w') as z:\n    z.writestr('../escape.txt', 'x')\n"
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg(script)
        .status()
        .expect("python3 runs");
    assert!(status.success());
    let destination = temp.path().join("out");
    let error = extract_archive(&escaping, &destination).expect_err("a parent path is refused");
    assert!(error.to_string().contains("outside"), "{error}");
    assert!(!destination.exists(), "nothing was written");
}

#[test]
fn extract_archive_refuses_what_it_cannot_unpack() {
    let temp = TempDir::new().expect("temp");
    let bogus = temp.path().join("notes.txt");
    fs::write(&bogus, "text").expect("file");
    let error = extract_archive(&bogus, temp.path()).expect_err("not an archive");
    assert!(error.to_string().contains("notes.txt"), "{error}");

    let broken = temp.path().join("broken.tar.gz");
    fs::write(&broken, "this is not gzip").expect("file");
    assert!(extract_archive(&broken, &temp.path().join("out")).is_err());
}
