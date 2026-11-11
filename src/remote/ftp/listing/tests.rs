//! Purpose: A Unix LIST row keeps its entry kind, its bare name and its permission bits.

use super::{parse, EntryKind};

#[test]
fn unix_list_preserves_directory_and_symlink_kind_and_permissions() {
    let rows = parse(
        &[
            "total 2".to_owned(),
            "drwxrwxr-x 1 root dialout 4096 Nov 5 2018 folder".to_owned(),
            "lrwxrwxrwx 1 root root 9 Nov 5 2018 link -> /tmp/target".to_owned(),
        ],
        false,
    )
    .expect("standard Unix listing");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind, EntryKind::Directory);
    assert_eq!(rows[0].attributes.bits(), 0o040_775);
    assert_eq!(rows[1].kind, EntryKind::SymlinkToFile);
    assert_eq!(rows[1].name, "link");
    assert_eq!(rows[1].attributes.bits(), 0o120_777);
}
