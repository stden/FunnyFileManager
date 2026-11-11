//! Purpose: unit tests for the counts a snapshot passes through: unreadable
//! processes and cores.

use super::*;

#[test]
fn the_unreadable_count_and_the_cores_pass_through() {
    let mut sample = first(Vec::new());
    sample.unreadable = 3;
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), sample);
    assert_eq!(snapshot.unreadable(), 3);
    assert_eq!(snapshot.cores(), CORES);
    assert_eq!(snapshot.rows().len(), 0);
}
