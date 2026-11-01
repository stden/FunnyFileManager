//! Purpose: A host that reports no cores draws no core bars.

use super::*;

#[test]
fn a_host_reporting_no_cores_shows_no_bars() {
    assert_eq!(core_bars(0, 4), (0, None));
}
