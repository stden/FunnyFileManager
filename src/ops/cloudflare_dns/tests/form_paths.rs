//! Purpose: The form's actions when no record is selected, when they are asked of the wrong view,
//! when a write is refused on the list, when a new record is checked, and when the host lies in
//! another zone.

use super::*;
use crate::ops::cloudflare_dns::form::{open_host, refused};
use crate::values::cloudflare_dns::{CloudflareError, CurrentHost};

#[test]
fn editing_with_no_record_selected_says_so() {
    let mut panel = CloudflareDnsPanel::new();
    edit_selected(&mut panel);
    assert_eq!(panel.status, "No record selected");
}

#[test]
fn review_and_apply_do_nothing_in_the_wrong_view() {
    let mut panel = CloudflareDnsPanel::new();
    review(&mut panel);
    apply(&mut panel);
    assert_eq!(panel.view.kind(), ViewKind::Browse);
    assert!(panel.busy.is_none());
}

#[test]
fn a_refused_write_on_the_list_leaves_the_list_and_says_why() {
    let mut panel = CloudflareDnsPanel::new();
    refused(&mut panel, &CloudflareError::Network);
    assert_eq!(panel.view.kind(), ViewKind::Browse);
    assert_eq!(panel.status, CloudflareError::Network.message());
}

#[test]
fn a_new_record_is_checked_into_its_confirmation() {
    let mut panel = CloudflareDnsPanel::new();
    let zone = zones().remove(0);
    let form = RecordForm::new_record(
        zone,
        "www".to_owned(),
        "203.0.113.5".to_owned(),
        String::new(),
    );
    panel.view = View::Edit(Box::new(form));
    review(&mut panel);
    assert_eq!(panel.view.kind(), ViewKind::Confirm);
}

#[test]
fn a_host_in_another_zone_selects_that_zone_and_reads_its_records() {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(zones());
    let host = CurrentHost {
        hostname: "host.example.net".to_owned(),
        ipv4: Ok("203.0.113.9".to_owned()),
    };
    open_host(&mut panel, host);
    assert_eq!(panel.busy, Some(Busy::Records));
    assert_eq!(
        panel.selected_zone().map(|zone| zone.name.as_str()),
        Some("example.net")
    );
}
