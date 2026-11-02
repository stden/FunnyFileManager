//! Image classification uses the file panel's cached entry kind.

use super::*;
use crate::models::{FolderTree, ImageState};

#[test]
fn png_uses_the_listed_file_kind_without_rechecking_the_path() {
    let dir = fixture();
    let path = dir.path().join("picture.png");
    fs::write(&path, b"image").expect("file");
    let mut app = app_in(&dir);
    select(&mut app, "picture.png");

    // A different object at the same path must not trigger a synchronous
    // stat while following the cursor. The image worker checks it later.
    fs::remove_file(&path).expect("remove file");
    fs::create_dir(&path).expect("replace with a directory");
    toggle(&mut app);

    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Image(ImageState::Pending)
    ));
    assert_eq!(
        crate::images::decode(&path).expect_err("a directory cannot be decoded"),
        "Not a regular file"
    );
}

#[test]
fn a_directory_named_like_an_image_is_still_a_folder() {
    let dir = fixture();
    fs::create_dir(dir.path().join("folder.png")).expect("folder");
    let mut app = app_in(&dir);
    select(&mut app, "folder.png");
    toggle(&mut app);

    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));
}

#[test]
fn a_tree_row_named_like_an_image_is_still_a_folder() {
    let dir = fixture();
    let folder = dir.path().join("folder.png");
    fs::create_dir(&folder).expect("folder");
    let mut app = app_in(&dir);
    app.left_panel.kind = PanelKind::Tree(Box::new(FolderTree::rooted_at(folder)));
    toggle(&mut app);

    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));
}

#[cfg(unix)]
#[test]
fn links_to_images_and_folders_keep_their_listed_target_kind() {
    use std::os::unix::fs::symlink;

    let dir = fixture();
    fs::write(dir.path().join("target.png"), b"image").expect("image");
    fs::create_dir(dir.path().join("target-dir")).expect("folder");
    symlink("target.png", dir.path().join("image-link.png")).expect("file link");
    symlink("target-dir", dir.path().join("folder-link.png")).expect("folder link");
    let mut app = app_in(&dir);
    select(&mut app, "image-link.png");
    toggle(&mut app);
    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Image(ImageState::Pending)
    ));

    select(&mut app, "folder-link.png");
    refresh(&mut app);
    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Folder(_)
    ));
}
