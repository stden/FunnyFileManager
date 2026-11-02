//! Unit tests: the user menu file sits beside the settings.

use super::{path, FILE_NAME};

#[test]
fn the_menu_file_lives_in_the_ffm_config_directory() {
    let Some(path) = path() else {
        return; // no home in this environment: nothing to locate
    };
    assert!(path.ends_with(std::path::Path::new("ffm").join(FILE_NAME)));
    assert_eq!(
        path.parent(),
        crate::config::Settings::path()
            .as_deref()
            .and_then(std::path::Path::parent),
        "beside settings.yaml"
    );
}
