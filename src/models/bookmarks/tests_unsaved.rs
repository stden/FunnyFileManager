//! Purpose: A store whose file has no folder to create reports the write error
//! instead of writing anywhere else.

use super::*;

#[test]
fn saving_to_a_file_with_no_folder_reports_the_error() {
    let bookmarks = Bookmarks {
        persistence: Persistence::File(PathBuf::new()),
        ..Bookmarks::default()
    };
    assert!(bookmarks.save().is_err());
}
