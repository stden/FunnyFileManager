//! Purpose: No write reaches Cloudflare before its confirmation is applied. The keys drive the real
//! dialog, the job runs the real request code, and a recording transport shows what was sent.

use super::*;
use crate::cloudflare_dns::{execute, Call, Method, Transport};
use crate::models::cloudflare_dns::{Proxy, Record, RecordId, Ttl, ViewKind};
use crate::popup::process_popup_input;
use crossterm::event::KeyCode;
use std::sync::Mutex;

type Sent = Arc<Mutex<Vec<Method>>>;

/// A transport that records the method of each call and answers with a refusal.
struct Recorder(Sent);

impl Transport for Recorder {
    fn send(&mut self, call: &Call) -> Result<serde_json::Value, CloudflareError> {
        self.0.lock().expect("the call log").push(call.method);
        Err(CloudflareError::Network)
    }
}

/// The fetch the job runs for one request, recording into `sent`.
fn recorder(sent: &Sent) -> impl FnOnce(Request) -> Reply + Send + 'static {
    let log = Arc::clone(sent);
    move |request| execute(&mut Recorder(log), request)
}

/// A dialog with one zone and one record, idle, on the record's form after two Enters.
fn on_the_form() -> CloudflareDnsPanel {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    }]);
    panel.set_records(vec![Record {
        id: RecordId::new("0000000000000000000000000000000b"),
        kind: "A".to_owned(),
        name: "www.example.com".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    }]);
    panel
}

#[test]
fn no_write_is_sent_until_the_confirmation_is_applied() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    app.set_overlay(Overlay::CloudflareDns(Box::new(on_the_form())));
    let sent: Sent = Arc::new(Mutex::new(Vec::new()));
    let mut jobs = CloudflareJobs::default();
    for code in [
        KeyCode::Enter,
        KeyCode::Enter,
        KeyCode::Char('9'),
        KeyCode::Enter,
    ] {
        process_popup_input(&mut app, code.into());
    }
    assert_eq!(panel(&app).view.kind(), ViewKind::Confirm);
    jobs.pump_with(&mut app, recorder(&sent));
    assert!(
        sent.lock().expect("the call log").is_empty(),
        "nothing is sent before the confirmation"
    );

    process_popup_input(&mut app, KeyCode::Enter.into());
    jobs.pump_with(&mut app, recorder(&sent));
    finish(&mut jobs, &mut app);
    assert_eq!(*sent.lock().expect("the call log"), [Method::Patch]);
}
