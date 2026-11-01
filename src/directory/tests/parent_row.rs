//! The .. row is dated from the folder above, whichever way it is read.

use super::*;
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn the_parent_row_is_dated_from_the_folder_above_not_from_this_one() -> io::Result<()> {
    let temp = TempDir::new()?;
    let folder = temp.path().join("child");
    fs::create_dir(&folder)?;
    // Date the child far in the past, so the parent's time cannot match it by chance.
    let long_ago = UNIX_EPOCH + Duration::from_secs(1_000_000);
    fs::File::open(&folder)?.set_modified(long_ago)?;
    // A later write in the parent moves its time to now.
    fs::write(temp.path().join("sibling.txt"), b"later")?;
    let parent = read(&folder, &[])?.parent.expect("parent row");
    assert_eq!(
        parent.modified,
        Modified::from_metadata(&fs::metadata(temp.path())?),
        "the .. row is dated from the folder it leads to"
    );
    assert_ne!(
        parent.modified,
        Modified::from_metadata(&fs::metadata(&folder)?),
        "the .. row is not dated from the folder it is shown in"
    );
    Ok(())
}
