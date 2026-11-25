//! Purpose: Draws the Cloudflare DNS dialog: the zone and record lists, the record form, or the
//! confirmation of one change. It reads the dialog's state and nothing else.

mod browse;
mod confirm;
mod form;
mod lines;
#[cfg(test)]
mod tests;

use crate::models::cloudflare_dns::{CloudflareDnsPanel, View};
use ratatui::Frame;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    panel: &CloudflareDnsPanel,
) {
    match &panel.view {
        View::Browse => browse::render(f, tracker, panel),
        View::Edit(form) => form::render(f, tracker, form, panel),
        View::Confirm(_, change) => confirm::render(f, tracker, change, panel),
    }
}
