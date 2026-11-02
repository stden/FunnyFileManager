//! Purpose: Draws the lists of the dialog: the zones, then the records of the selected zone, the
//! status line and the keys.

use super::lines::{record_line, status, window, zone_line};
use crate::models::cloudflare_dns::{CloudflareDnsPanel, Focus};
use crate::ui::dialog::{self, Ground, Row};
use ratatui::Frame;

const TITLE: &str = "Cloudflare DNS";
const KEYS: &str =
    "Up/Down move | Tab zone/records | Enter edit | a add this host | r reload | Esc close";
const ZONE_ROWS: usize = 6;
const RECORD_ROWS: usize = 10;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    panel: &CloudflareDnsPanel,
) {
    let zones = zone_lines(panel);
    let caption = panel.selected_zone().map_or_else(
        || "Records".to_owned(),
        |zone| format!("Records of {}", zone.name),
    );
    let records = record_lines(panel);
    let status = status(panel);
    let mut rows = vec![Row::Label("Zones")];
    rows.extend(zones.iter().map(|line| Row::Plain(line.as_str())));
    rows.extend([Row::Divider, Row::Label(&caption)]);
    rows.extend(records.iter().map(|line| Row::Plain(line.as_str())));
    rows.extend([Row::Divider, Row::Plain(&status), Row::Plain(KEYS)]);
    dialog::render_tracked(f, tracker, TITLE, Ground::Prompt, &rows, &[]);
}

fn zone_lines(panel: &CloudflareDnsPanel) -> Vec<String> {
    if panel.zones.is_empty() {
        return vec!["No zones".to_owned()];
    }
    let focused = panel.focus == Focus::Zones;
    let shown = window(panel.zones.len(), panel.zone, ZONE_ROWS);
    panel
        .zones
        .iter()
        .enumerate()
        .skip(shown.start)
        .take(shown.len())
        .map(|(index, zone)| zone_line(zone, focused && index == panel.zone))
        .collect()
}

fn record_lines(panel: &CloudflareDnsPanel) -> Vec<String> {
    if panel.records.is_empty() {
        return vec!["No records in this zone".to_owned()];
    }
    let focused = panel.focus == Focus::Records;
    let shown = window(panel.records.len(), panel.record, RECORD_ROWS);
    panel
        .records
        .iter()
        .enumerate()
        .skip(shown.start)
        .take(shown.len())
        .map(|(index, record)| record_line(record, focused && index == panel.record))
        .collect()
}
