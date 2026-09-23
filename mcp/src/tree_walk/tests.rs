use super::{directory_size, render_directory_tree};
use std::fs;
use tempfile::TempDir;

#[test]
fn directory_size_totals_nested_files() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir_all(temp.path().join("nested")).expect("nested dir");
    fs::write(temp.path().join("root.bin"), b"1234").expect("root file");
    fs::write(temp.path().join("nested/child.bin"), b"123456").expect("child file");

    assert_eq!(directory_size(temp.path()), 10);
}

#[test]
fn directory_size_of_a_file_is_its_length() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("payload.bin");
    fs::write(&file, [0_u8; 257]).expect("file");

    assert_eq!(directory_size(&file), 257);
}

#[test]
fn empty_directory_has_no_size_and_only_renders_the_root() {
    let temp = TempDir::new().expect("temp dir");

    assert_eq!(directory_size(temp.path()), 0);
    assert_eq!(
        render_directory_tree(temp.path(), usize::MAX),
        format!("{}/", temp.path().display())
    );
}

#[test]
fn render_directory_tree_sorts_entries_and_honours_depth() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir_all(temp.path().join("alpha/deeper")).expect("nested dirs");
    fs::write(temp.path().join("zeta.txt"), b"z").expect("file");
    fs::write(temp.path().join("alpha/child.txt"), b"a").expect("child file");

    let tree = render_directory_tree(temp.path(), 1);
    let rows: Vec<_> = tree.lines().skip(1).collect();

    assert_eq!(rows, ["├── alpha/", "└── zeta.txt"]);
}

#[test]
fn render_directory_tree_reuses_the_correct_nested_prefix() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir_all(temp.path().join("alpha/deeper")).expect("nested dirs");
    fs::write(temp.path().join("alpha/child.txt"), b"a").expect("child file");
    fs::write(temp.path().join("zeta.txt"), b"z").expect("root file");

    let tree = render_directory_tree(temp.path(), 2);
    let rows: Vec<_> = tree.lines().skip(1).collect();

    assert_eq!(
        rows,
        [
            "├── alpha/",
            "│   ├── child.txt",
            "│   └── deeper/",
            "└── zeta.txt"
        ]
    );
}

#[test]
fn zero_depth_renders_no_entries() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("directory")).expect("directory");
    fs::write(temp.path().join("file.txt"), b"contents").expect("file");

    assert_eq!(
        render_directory_tree(temp.path(), 0),
        format!("{}/", temp.path().display())
    );
}

#[test]
fn missing_paths_have_no_size_and_only_render_the_root() {
    let temp = TempDir::new().expect("temp dir");
    let missing = temp.path().join("missing");

    assert_eq!(directory_size(&missing), 0);
    assert_eq!(
        render_directory_tree(&missing, usize::MAX),
        format!("{}/", missing.display())
    );
}

#[cfg(unix)]
#[test]
fn directory_symlink_is_counted_and_rendered_as_a_leaf() {
    let root = TempDir::new().expect("root temp dir");
    let outside = TempDir::new().expect("outside temp dir");
    fs::write(outside.path().join("large.bin"), vec![0_u8; 64 * 1024]).expect("outside file");
    let link = root.path().join("link");
    std::os::unix::fs::symlink(outside.path(), &link).expect("directory link");
    let link_size = fs::symlink_metadata(&link).expect("link metadata").len();

    assert_eq!(directory_size(root.path()), link_size);
    assert_eq!(
        render_directory_tree(root.path(), usize::MAX)
            .lines()
            .skip(1)
            .collect::<Vec<_>>(),
        ["└── link"]
    );
}
