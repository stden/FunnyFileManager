//! Diagnostic table layout with no samples and with a narrow terminal.

use super::{section_lines, ProbeColumns};
use crate::models::tasks::NetDiagnostics;
use crate::ui::Cells;

#[test]
fn an_empty_diagnostics_table_keeps_headers_and_pads_its_body() {
    let width = Cells::new(80);
    let columns = ProbeColumns::fit(width);
    let lines = section_lines(&NetDiagnostics::default(), &columns, width, 4);
    assert_eq!(lines.len(), 4);
    assert!(lines[0].to_string().contains("Latency & DNS Diagnostics"));
    assert!(lines[1].to_string().contains("Target"));
    assert_eq!(lines[2], lines[3]);
    assert!(!lines[2].to_string().contains("Cloudflare"));
}

#[test]
fn a_narrow_table_clips_its_caption_to_the_available_width() {
    let width = Cells::new(10);
    let columns = ProbeColumns::fit(width);
    let lines = section_lines(&NetDiagnostics::default(), &columns, width, 3);
    assert_eq!(lines.len(), 3);
    assert!(Cells::of(&lines[0].to_string()).get() <= width.get());
}
