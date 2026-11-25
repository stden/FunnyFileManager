//! Purpose: A dialog's request waits while another request of the job is still in flight.

use super::*;

#[test]
fn a_request_waits_while_another_is_in_flight() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    let mut dialog = CloudflareDnsPanel::new();
    dialog.enqueue(Request::Zones);
    app.set_overlay(Overlay::CloudflareDns(Box::new(dialog)));
    let (release, gate) = channel::channel();
    let mut jobs = CloudflareJobs::default();
    jobs.pump_with(&mut app, held(gate));
    if let Overlay::CloudflareDns(panel) = &mut app.overlay {
        panel.settle();
        panel.enqueue(Request::Zones);
    }
    jobs.pump_with(&mut app, |_| {
        panic!("the second request waits for the first")
    });
    assert!(
        jobs.pending.is_some(),
        "the first request is still in flight"
    );
    release.send(()).expect("the first request is released");
    finish(&mut jobs, &mut app);
}
