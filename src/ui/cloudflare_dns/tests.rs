//! Purpose: What the dialog draws: its lists and status line, the note on a form, the exact lines of
//! a confirmation, and a list window that keeps its cursor on screen.

use super::lines::window;
use crate::models::cloudflare_dns::{
    Change, CloudflareDnsPanel, Proxy, Record, RecordForm, RecordId, RecordUpdate, Request, Ttl,
    View, Zone, ZoneId,
};
use crate::models::{App, Overlay, Panel};

fn zone() -> Zone {
    Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    }
}

fn record() -> Record {
    Record {
        id: RecordId::new("0000000000000000000000000000000b"),
        kind: "A".to_owned(),
        name: "www.example.com".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    }
}

fn loaded() -> CloudflareDnsPanel {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![zone()]);
    panel.set_records(vec![record()]);
    panel
}

fn app_showing(panel: CloudflareDnsPanel, temp: &tempfile::TempDir) -> App {
    let list = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(list.clone(), list);
    app.set_overlay(Overlay::CloudflareDns(Box::new(panel)));
    app
}

fn render_text(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, app))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[test]
fn the_browse_view_lists_the_zones_and_the_records_of_the_selected_zone() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_showing(loaded(), &temp);
    let text = render_text(&mut app);
    assert!(text.contains("Cloudflare DNS"));
    assert!(text.contains("example.com"));
    assert!(text.contains("203.0.113.5"));
    assert!(text.contains("Esc close"));
}

#[test]
fn the_loading_line_says_what_is_awaited() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut panel = loaded();
    panel.enqueue(Request::Zones);
    let mut app = app_showing(panel, &temp);
    assert!(render_text(&mut app).contains("Loading zones"));
}

#[test]
fn a_confirmation_shows_the_exact_change_it_sends() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut panel = loaded();
    let update = RecordUpdate {
        zone: zone(),
        record: record(),
        content: "203.0.113.9".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    };
    let form = RecordForm::edit(zone(), record());
    panel.view = View::Confirm(Box::new(form), Box::new(Change::Update(update)));
    let mut app = app_showing(panel, &temp);
    let text = render_text(&mut app);
    assert!(text.contains("content: 203.0.113.5 -> 203.0.113.9"));
    assert!(text.contains("Enter apply this change"));
}

#[test]
fn a_form_says_why_its_address_is_empty() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut panel = loaded();
    let note = "Public IPv4 not detected: the network request failed";
    let form = RecordForm::new_record(zone(), "host".to_owned(), String::new(), note.to_owned());
    panel.view = View::Edit(Box::new(form));
    let mut app = app_showing(panel, &temp);
    assert!(render_text(&mut app).contains("Public IPv4 not detected"));
}

#[test]
fn the_list_window_keeps_the_cursor_on_screen() {
    assert_eq!(window(3, 0, 5), 0..3);
    let shown = window(20, 15, 5);
    assert_eq!(shown.len(), 5);
    assert!(shown.contains(&15));
    assert!(window(20, 19, 5).contains(&19));
}
