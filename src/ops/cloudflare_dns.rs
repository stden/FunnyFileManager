//! Purpose: The Cloudflare DNS dialog's actions. It opens with the zone list, takes the answers of
//! its requests, and its keys move the lists and start the loads. Writes live in `form`.

mod form;
#[cfg(test)]
mod tests;

pub use form::{apply, back, edit_selected, review};

use crate::models::cloudflare_dns::{CloudflareDnsPanel, Reply, Request};
use crate::models::{App, Overlay};

pub fn open(app: &mut App) {
    let mut panel = CloudflareDnsPanel::new();
    start(&mut panel, Request::Zones);
    app.set_overlay(Overlay::CloudflareDns(Box::new(panel)));
}

/// Applies the answer to the request the dialog waits for: the lists it loads, and the form or
/// the list it returns to.
pub fn publish(panel: &mut CloudflareDnsPanel, reply: Reply) {
    panel.settle();
    match reply {
        Reply::Zones(Ok(zones)) => {
            panel.status = format!("{} zones", zones.len());
            if let Some(zone) = panel.set_zones(zones) {
                start(panel, Request::Records(zone));
            }
        }
        Reply::Records(zone, Ok(records)) => {
            if panel
                .selected_zone()
                .is_some_and(|selected| selected.id == zone)
            {
                panel.set_records(records);
            }
        }
        Reply::Host(Ok(host)) => form::open_host(panel, host),
        Reply::Written(Ok(record)) => form::saved(panel, &record),
        Reply::Written(Err(error)) => form::refused(panel, &error),
        Reply::Zones(Err(error)) | Reply::Records(_, Err(error)) | Reply::Host(Err(error)) => {
            panel.status = error.message();
        }
    }
}

/// Up or Down in the focused list. Moving to another zone loads its records.
pub fn step(panel: &mut CloudflareDnsPanel, forward: bool) {
    if let Some(zone) = panel.step(forward) {
        start(panel, Request::Records(zone));
    }
}

/// `r`: the zone list is read again, and then the records of the zone that is selected.
pub fn reload(panel: &mut CloudflareDnsPanel) {
    start(panel, Request::Zones);
}

/// `a`: this machine's hostname and public address, as a new record.
pub fn add_host(panel: &mut CloudflareDnsPanel) {
    start(panel, Request::Host);
}

/// Queues `request`. A dialog that is already waiting for an answer queues nothing more.
fn start(panel: &mut CloudflareDnsPanel, request: Request) {
    if !panel.enqueue(request) {
        panel.status = String::from("Still loading: wait for the current request");
    }
}
