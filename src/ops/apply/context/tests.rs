use super::*;
use std::path::Path;

#[test]
fn display_path_strips_windows_prefixes() {
    assert_eq!(
        display_path(Path::new(r"\\?\UNC\server\share")),
        r"\\server\share"
    );
    assert_eq!(display_path(Path::new(r"\\?\C:\Windows")), r"C:\Windows");
    assert_eq!(drive(Path::new("/")), std::path::MAIN_SEPARATOR.to_string());
    assert_eq!(drive(Path::new("relative")), "");
}
