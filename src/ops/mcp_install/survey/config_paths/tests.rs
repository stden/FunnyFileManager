//! Purpose: Path resolution fallback when no ancestor exists on disk.

use std::path::{Path, PathBuf};

use super::resolved;

#[test]
fn resolved_returns_the_original_path_when_no_ancestor_exists() {
    let relative = Path::new("definitely_missing_dir_xyz/config.json");
    assert_eq!(resolved(relative), PathBuf::from(relative));
}
