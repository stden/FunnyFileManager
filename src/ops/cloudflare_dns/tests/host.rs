//! Purpose: `a` asks for this machine's host, one request at a time.

use super::*;

#[test]
fn adding_the_host_queues_one_host_request_and_says_so_while_it_waits() {
    let mut panel = CloudflareDnsPanel::new();
    add_host(&mut panel);
    assert_eq!(panel.busy, Some(Busy::Host));
    add_host(&mut panel);
    assert_eq!(panel.status, "Still loading: wait for the current request");
}
