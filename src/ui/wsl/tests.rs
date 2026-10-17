//! WSL frames render state and remain useful on a narrow terminal.

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::models::{DistributionName, WslRequestId};

use super::*;

fn dialog(names: &[&str]) -> WslDialog {
    WslDialog::new(
        names
            .iter()
            .map(|name| DistributionName::new(name).expect("distribution"))
            .collect(),
    )
}

fn draw(dialog: &WslDialog, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| render(frame, dialog))
        .expect("render");
    terminal.backend().buffer().clone()
}

fn screen(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn normal_window_names_both_update_operations_and_the_selected_distribution() {
    let mut dialog = dialog(&["Ubuntu", "Debian"]);
    dialog.move_down();
    let buffer = draw(&dialog, 90, 24);
    let text = screen(&buffer);
    for label in [
        "WSL",
        "Ubuntu",
        "Debian",
        "Enter Open root",
        "F2 Refresh list",
        "F5 Update WSL",
        "F6 Update packages",
        "Esc Close",
    ] {
        assert!(text.contains(label), "missing {label}:\n{text}");
    }
    let selected = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "D")
        .expect("Debian");
    assert_eq!(selected.bg, palette::MENU_SELECTED_BG);
    assert_eq!(selected.fg, palette::DRIVE_FG);
    assert!(buffer
        .content()
        .iter()
        .any(|cell| cell.bg == palette::MENU_BG));
}

#[test]
fn selection_at_the_end_of_a_long_list_stays_on_screen() {
    let names: Vec<String> = (0..30).map(|index| format!("Distro{index:02}")).collect();
    let mut dialog = dialog(&names.iter().map(String::as_str).collect::<Vec<_>>());
    dialog.last();
    let text = screen(&draw(&dialog, 42, 12));
    assert!(text.contains("Distro29"), "{text}");
    assert!(
        !text.contains("Distro00"),
        "top rows must yield to selection: {text}"
    );
}

#[test]
fn narrow_window_keeps_failure_selected_distribution_and_close_key_visible() {
    let mut dialog = dialog(&["Ubuntu", "Debian"]);
    dialog.move_down();
    dialog.activity = WslActivity::Failed("WSL update unavailable".to_owned());
    let text = screen(&draw(&dialog, 18, 6));
    assert!(text.contains("Error:"), "{text}");
    assert!(text.contains("Debian"), "{text}");
    assert!(text.contains("Esc"), "{text}");
}

#[test]
fn busy_status_identifies_platform_or_selected_package_update() {
    for (operation, expected) in [
        (WslOperation::Refresh, "Refreshing distributions"),
        (WslOperation::Update, "Updating WSL"),
        (
            WslOperation::UpdatePackages(DistributionName::new("Debian").expect("name")),
            "Updating packages: Debian",
        ),
        (
            WslOperation::Open(DistributionName::new("Debian").expect("name")),
            "Opening Debian",
        ),
    ] {
        let mut dialog = dialog(&["Debian"]);
        dialog.activity = WslActivity::Busy {
            id: WslRequestId::new(1),
            operation,
        };
        let text = screen(&draw(&dialog, 90, 24));
        assert!(text.contains(expected), "{text}");
    }
}

#[test]
fn empty_lists_notices_and_tiny_terminals_render_without_external_queries() {
    let mut dialog = dialog(&[]);
    assert!(screen(&draw(&dialog, 60, 16)).contains("No distributions"));
    dialog.activity = WslActivity::Notice("WSL is up to date".to_owned());
    assert!(screen(&draw(&dialog, 60, 16)).contains("WSL is up to date"));
    for (width, height) in [(0, 0), (1, 1), (2, 2), (3, 3), (8, 4)] {
        let buffer = draw(&dialog, width, height);
        assert_eq!(buffer.area.width, width);
        assert_eq!(buffer.area.height, height);
    }
}

fn locate(buffer: &Buffer, label: &str) -> (u16, u16) {
    for row in 0..buffer.area.height {
        let text: String = (0..buffer.area.width)
            .map(|column| buffer[(column, row)].symbol())
            .collect();
        if let Some((before, _)) = text.split_once(label) {
            return (
                u16::try_from(Cells::of(before).get()).expect("screen column"),
                row,
            );
        }
    }
    panic!("missing {label}:\n{}", screen(buffer));
}

#[test]
fn every_visible_button_click_matches_its_rendered_key_after_resize() {
    let dialog = dialog(&["Debian"]);
    for (width, height) in [(90, 24), (42, 12), (18, 6)] {
        let buffer = draw(&dialog, width, height);
        for (label, command) in [
            ("Open", WslCommand::Open),
            ("F2", WslCommand::Refresh),
            ("F5", WslCommand::Update),
            ("F6", WslCommand::UpdatePackages),
            ("Esc", WslCommand::Close),
        ] {
            let (column, row) = locate(&buffer, label);
            assert_eq!(
                wsl_button_at(buffer.area, &dialog, column, row),
                Some(command),
                "{label} at {width}x{height}"
            );
        }
        let (column, row) = locate(&buffer, "Debian");
        assert_eq!(wsl_button_at(buffer.area, &dialog, column, row), None);
        assert_eq!(wsl_button_at(buffer.area, &dialog, 0, 0), None);
        assert_eq!(wsl_button_at(buffer.area, &dialog, width, height), None);
    }
}

#[test]
fn busy_buttons_are_dim_and_disabled_while_close_stays_clickable() {
    let mut dialog = dialog(&["Debian"]);
    dialog.activity = WslActivity::Busy {
        id: WslRequestId::new(1),
        operation: WslOperation::Update,
    };
    for (width, height) in [(90, 24), (18, 6)] {
        let buffer = draw(&dialog, width, height);
        for label in ["Open", "F2", "F5", "F6"] {
            let (column, row) = locate(&buffer, label);
            assert_eq!(wsl_button_at(buffer.area, &dialog, column, row), None);
            assert_eq!(buffer[(column, row)].fg, palette::DISABLED_FG);
        }
        let (column, row) = locate(&buffer, "Esc");
        assert_eq!(
            wsl_button_at(buffer.area, &dialog, column, row),
            Some(WslCommand::Close)
        );
        assert_eq!(buffer[(column, row)].fg, palette::HOTKEY);
    }
}
