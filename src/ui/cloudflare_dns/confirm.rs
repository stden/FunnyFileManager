//! Purpose: Draws the confirmation of one change: the exact lines the change sends, in the warning
//! colour, with the keys that apply it or go back.

use super::lines::status;
use crate::models::cloudflare_dns::{Change, CloudflareDnsPanel};
use crate::ui::dialog::{self, Ground, Row};
use ratatui::Frame;

const TITLE: &str = "Confirm change";
const KEYS: &str = "Enter apply this change | Esc back to the form";

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    change: &Change,
    panel: &CloudflareDnsPanel,
) {
    let lines = change.lines();
    let status = status(panel);
    let mut rows: Vec<Row<'_>> = lines.iter().map(|line| Row::Plain(line.as_str())).collect();
    rows.extend([Row::Divider, Row::Plain(&status), Row::Plain(KEYS)]);
    dialog::render_tracked(f, tracker, TITLE, Ground::Warning, &rows, &[]);
}
