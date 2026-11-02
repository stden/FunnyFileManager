//! Purpose: What the dialog's actions do to its state: the loads that follow a list, the host form,
//! validation before confirmation, and a write only after it is confirmed.

use super::*;
use crate::models::cloudflare_dns::{
    Busy, Proxy, Record, RecordForm, RecordId, Ttl, View, ViewKind, Zone, ZoneId,
};
use crate::models::Panel;

mod answers;
mod confirm;

pub(super) const ZONE_A: &str = "0000000000000000000000000000000a";
pub(super) const ZONE_B: &str = "0000000000000000000000000000000c";

pub(super) fn zones() -> Vec<Zone> {
    vec![
        Zone {
            id: ZoneId::new(ZONE_A),
            name: "example.com".to_owned(),
        },
        Zone {
            id: ZoneId::new(ZONE_B),
            name: "example.net".to_owned(),
        },
    ]
}

pub(super) fn record(name: &str, content: &str) -> Record {
    Record {
        id: RecordId::new("0000000000000000000000000000000b"),
        kind: "A".to_owned(),
        name: name.to_owned(),
        content: content.to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    }
}

/// A dialog that has its zones and the records of the first zone.
pub(super) fn loaded() -> CloudflareDnsPanel {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(zones());
    panel.set_records(vec![record("www.example.com", "203.0.113.5")]);
    panel
}

/// The form of the one loaded record, as the Enter key opens it.
pub(super) fn edit_form() -> RecordForm {
    RecordForm::edit(zones().remove(0), record("www.example.com", "203.0.113.5"))
}

/// The dialog's state with `form` open for review.
pub(super) fn editing(form: RecordForm) -> CloudflareDnsPanel {
    let mut panel = loaded();
    panel.view = View::Edit(Box::new(form));
    panel
}

pub(super) fn app(temp: &tempfile::TempDir) -> App {
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    App::with_panels(panel.clone(), panel)
}

#[test]
fn opening_the_dialog_queues_the_zone_list_and_shows_it_loading() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    open(&mut app);
    let Overlay::CloudflareDns(panel) = &app.overlay else {
        panic!("the dialog");
    };
    assert_eq!(panel.busy, Some(Busy::Zones));
    assert_eq!(panel.queued, Some(Request::Zones));
}

#[test]
fn the_dialog_keeps_its_view_kind_for_the_keys_to_read() {
    assert_eq!(CloudflareDnsPanel::new().view.kind(), ViewKind::Browse);
    assert_eq!(
        editing(edit_form()).view.kind(),
        ViewKind::Edit,
        "a form is an edit view"
    );
}
