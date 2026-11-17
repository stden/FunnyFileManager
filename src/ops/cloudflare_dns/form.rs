//! Purpose: The record form's actions: opening it on a record or on this host, checking it, asking
//! for its confirmation, and applying the answer to a confirmed write. Nothing is written before
//! `apply`, and `apply` only queues the request.

use super::start;
use crate::cloudflare_dns as cloudflare;
use crate::models::cloudflare_dns::{
    Change, CloudflareDnsPanel, CloudflareError, CurrentHost, NewRecord, Record, RecordForm,
    RecordUpdate, Request, Target, View,
};

/// Enter on a record in the list: its form, with the record's current values.
pub fn edit_selected(panel: &mut CloudflareDnsPanel) {
    let (Some(zone), Some(record)) = (
        panel.selected_zone().cloned(),
        panel.selected_record().cloned(),
    ) else {
        panel.status = String::from("No record selected");
        return;
    };
    panel.view = View::Edit(Box::new(RecordForm::edit(zone, record)));
}

/// Enter on the form: a changed record is checked and becomes its confirmation. Nothing is sent.
pub fn review(panel: &mut CloudflareDnsPanel) {
    let View::Edit(form) = &panel.view else {
        return;
    };
    match change_of(form) {
        Ok(change) => {
            let form = form.clone();
            panel.view = View::Confirm(form, Box::new(change));
        }
        Err(error) => panel.status = error.message(),
    }
}

/// Enter on the confirmation: the one write the dialog was asked to make.
pub fn apply(panel: &mut CloudflareDnsPanel) {
    let View::Confirm(_, change) = &panel.view else {
        return;
    };
    let request = Request::Write((**change).clone());
    start(panel, request);
}

/// Esc: a confirmation returns to its form, and a form returns to the list.
pub fn back(panel: &mut CloudflareDnsPanel) {
    panel.view = match std::mem::replace(&mut panel.view, View::Browse) {
        View::Confirm(form, _) => View::Edit(form),
        View::Edit(_) | View::Browse => View::Browse,
    };
}

/// The answer to the host lookup: a new record under the zone the host is in, with the address
/// that was found, or with the reason none was.
pub(super) fn open_host(panel: &mut CloudflareDnsPanel, host: CurrentHost) {
    let Some(zone) = cloudflare::choose(&panel.zones, &host.hostname).cloned() else {
        panel.status = CloudflareError::NoZone(host.hostname).message();
        return;
    };
    let name = cloudflare::relative_name(&host.hostname, &zone);
    let (content, note) = match host.ipv4 {
        Ok(ip) => (ip, String::new()),
        Err(error) => (
            String::new(),
            format!("Public IPv4 not detected: {}", error.message()),
        ),
    };
    if let Some(index) = panel
        .zone_index(&zone.id)
        .filter(|index| *index != panel.zone)
    {
        if let Some(id) = panel.select_zone(index) {
            start(panel, Request::Records(id));
        }
    }
    panel.view = View::Edit(Box::new(RecordForm::new_record(zone, name, content, note)));
}

/// A saved write: the list comes back, and the zone's records are read again.
pub(super) fn saved(panel: &mut CloudflareDnsPanel, record: &Record) {
    panel.view = View::Browse;
    panel.status = format!("Saved {} {}", record.name, record.content);
    if let Some(zone) = panel.selected_zone().map(|zone| zone.id.clone()) {
        start(panel, Request::Records(zone));
    }
}

/// A write that failed: the form comes back, with the reason.
pub(super) fn refused(panel: &mut CloudflareDnsPanel, error: &CloudflareError) {
    panel.view = match std::mem::replace(&mut panel.view, View::Browse) {
        View::Confirm(form, _) | View::Edit(form) => View::Edit(form),
        View::Browse => View::Browse,
    };
    panel.status = error.message();
}

/// The change the form asks for, once every value is checked.
fn change_of(form: &RecordForm) -> Result<Change, CloudflareError> {
    let ttl = cloudflare::ttl(&form.ttl)?;
    match &form.target {
        Target::New => {
            let name = cloudflare::record_name(&form.name)?;
            let content = cloudflare::content("A", &form.content)?;
            cloudflare::proxy("A", form.proxy)?;
            Ok(Change::Create(NewRecord {
                zone: form.zone.clone(),
                name,
                content,
                ttl,
                proxy: form.proxy,
            }))
        }
        Target::Existing(record) => {
            let content = cloudflare::content(&record.kind, &form.content)?;
            cloudflare::proxy(&record.kind, form.proxy)?;
            let update = RecordUpdate {
                zone: form.zone.clone(),
                record: record.clone(),
                content,
                ttl,
                proxy: form.proxy,
            };
            if update.changed().is_empty() {
                return Err(CloudflareError::NoChange);
            }
            Ok(Change::Update(update))
        }
    }
}
