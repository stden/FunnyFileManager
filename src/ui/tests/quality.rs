//! Objective UI readability contracts and optional SVG artifacts for visual review.

use std::{collections::BTreeSet, path::Path};

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::menu::SettingsPage;

mod contrast;
mod fixtures;
mod scope;
mod snapshot_tests;

mod color_tests;
#[path = "../../../examples/support/snapshot.rs"]
mod snapshot;
use snapshot::colors;

const VIEWPORTS: [(u16, u16); 3] = [(40, 12), (80, 24), (120, 30)];
const THEMES: [(&str, &str); 6] = [
    ("blue", "on"),
    ("black", "on"),
    ("gray", "on"),
    ("blue", "off"),
    ("black", "off"),
    ("gray", "off"),
];

#[test]
fn rendered_screens_meet_visual_quality_contract() {
    let mut failures = Vec::new();
    let mut rendered = 0;
    for (background, highlighting) in THEMES {
        for (width, height) in VIEWPORTS {
            let (errors, count) = check_viewport(background, highlighting, width, height);
            failures.extend(errors);
            rendered += count;
        }
    }
    save_report(&failures, rendered);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn check_viewport(
    background: &str,
    highlighting: &str,
    width: u16,
    height: u16,
) -> (Vec<String>, usize) {
    let mut names = BTreeSet::new();
    let mut failures = Vec::new();
    for mut case in fixtures::cases() {
        assert!(
            names.insert(case.name),
            "duplicate UI fixture: {}",
            case.name
        );
        let buffer = render(&mut case, background, highlighting, width, height);
        let context = format!(
            "{} {width}x{height} {background} highlighting={highlighting}",
            case.name
        );
        save_artifact(
            &buffer,
            &context,
            case.name == "panels" || (background == "blue" && highlighting == "on"),
        );
        if let Some(error) = content_failure(&buffer, &case, width, height) {
            failures.push(format!("{context}: {error}"));
        }
        let active = scope::active_text(&buffer, &mut case.app);
        failures.extend(
            contrast::check(&active)
                .into_iter()
                .map(|error| format!("{context}: {error}")),
        );
    }
    assert!(!names.is_empty(), "UI fixtures must be discovered");
    (failures, names.len())
}

fn render(
    case: &mut fixtures::Case,
    background: &str,
    highlighting: &str,
    width: u16,
    height: u16,
) -> Buffer {
    case.app
        .settings
        .set(SettingsPage::Colors, background)
        .unwrap();
    case.app
        .settings
        .set(SettingsPage::Highlighting, highlighting)
        .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| super::super::ui(frame, &mut case.app))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn content_failure(
    buffer: &Buffer,
    case: &fixtures::Case,
    width: u16,
    height: u16,
) -> Option<String> {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fixtures::validate(buffer, case, width, height);
    }))
    .err()?;
    Some(error.downcast_ref::<String>().cloned().unwrap_or_else(|| {
        error
            .downcast_ref::<&str>()
            .copied()
            .unwrap_or("layout validation panicked")
            .to_owned()
    }))
}

fn save_report(failures: &[String], rendered: usize) {
    let Some(directory) = std::env::var_os("FFM_UI_ARTIFACT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    let result = if failures.is_empty() {
        "UI contrast, text and layout checks passed. SVGs require human visual review.\n".to_owned()
    } else {
        failures.join("\n")
    };
    let report = format!(
        "{rendered} frames: {} viewports, {} theme configurations.\n{result}",
        VIEWPORTS.len(),
        THEMES.len()
    );
    std::fs::write(Path::new(&directory).join("report.txt"), report).unwrap();
}

fn save_artifact(buffer: &Buffer, context: &str, selected: bool) {
    let Some(directory) = std::env::var_os("FFM_UI_ARTIFACT_DIR") else {
        return;
    };
    if !selected {
        return;
    }
    let name: String = context
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let output = Path::new(&directory).join(format!("{name}.svg"));
    snapshot::save(buffer, context, &output).expect("save UI visual-review artifact");
}
