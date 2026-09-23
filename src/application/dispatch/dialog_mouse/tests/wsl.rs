//! Real rendered button clicks pass through mouse dispatch, the DSL and operations.

use ratatui::Terminal;
use unicode_width::UnicodeWidthStr;

use crate::application::dispatch::dispatch_mouse;
use crate::models::{DistributionName, WslActivity, WslDialog, WslOperation, WslOutput};
use crate::mouse::{PanelClicks, Target};

use super::{
    app_at, draw, mouse, App, MouseButton, MouseEventKind, Overlay, PanelSide, TempDir,
    WritableTestBackend,
};

fn open_dialog(app: &mut App) {
    let mut dialog = WslDialog::new(
        ["Ubuntu", "Debian", "Alpine"]
            .into_iter()
            .map(|name| DistributionName::new(name).expect("distribution"))
            .collect(),
    );
    dialog.move_down();
    app.set_overlay(Overlay::Wsl(Box::new(dialog)));
}

fn click_label(app: &mut App, terminal: &mut Terminal<WritableTestBackend>, label: &str) {
    draw(app, terminal);
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let position = (0..area.height)
        .find_map(|row| {
            let text: String = (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect();
            text.split_once(label)
                .map(|(before, _)| {
                    let column = u16::try_from(before.width()).expect("screen column");
                    (column, row)
                })
                .filter(|&(column, row)| {
                    // Only dialog buttons, not a random "F5" in the background panel path.
                    buffer[(column, row)].bg == ratatui::style::Color::Rgb(0, 128, 128)
                })
        })
        .expect("button label must be rendered");
    dispatch_mouse(
        terminal,
        app,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            position.0,
            position.1,
        ),
        area,
        &mut PanelClicks::default(),
    );
}

#[test]
fn wsl_rendered_update_buttons_queue_distinct_operations_and_busy_close_stays_active() {
    let dir = TempDir::new().expect("temp");
    std::fs::write(dir.path().join("kept.txt"), "kept").expect("file");
    for (width, height) in [(80, 24), (18, 6)] {
        let mut app = app_at(&dir);
        let mut terminal =
            Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
        open_dialog(&mut app);

        queue_system_update(&mut app, &mut terminal);
        queue_package_update(&mut app, &mut terminal);
        click_label(&mut app, &mut terminal, "Esc");
        assert!(matches!(app.overlay, Overlay::None));
        assert_eq!(
            app.wsl_requests.pending().len(),
            1,
            "closing must retain pending background work"
        );
        assert_eq!(app.left_panel.path, dir.path());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("kept.txt")).expect("original file"),
            "kept"
        );
    }
}

#[test]
fn wsl_overlay_consumes_clicks_on_the_visible_panel_behind_it() {
    let dir = TempDir::new().expect("temp");
    std::fs::write(dir.path().join("kept.txt"), "kept").expect("file");
    let mut app = app_at(&dir);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    app.left_panel.select_named("kept.txt");
    open_dialog(&mut app);
    draw(&mut app, &mut terminal);
    click_panel_behind_dialog(&mut app, &mut terminal);
    assert_eq!(
        app.active_panel,
        PanelSide::Left,
        "modal click must not activate the right panel"
    );
    assert!(matches!(&app.overlay, Overlay::Wsl(dialog)
        if dialog.selected_distribution().map(DistributionName::as_str) == Some("Debian")));
    assert_eq!(
        app.left_panel.get_selected_entry().expect("selection").name,
        "kept.txt"
    );
    assert!(app.wsl_requests.pending().is_empty());
    assert!(dir.path().join("kept.txt").exists());
}

fn queue_system_update(app: &mut App, terminal: &mut Terminal<WritableTestBackend>) {
    click_label(app, terminal, "F5");
    let queued = app.wsl_requests.take();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].operation, WslOperation::Update);
    assert!(matches!(&app.overlay, Overlay::Wsl(dialog)
            if matches!(dialog.activity, WslActivity::Busy { operation: WslOperation::Update, .. })));
    crate::ops::wsl::complete(
        app,
        queued[0].id,
        Ok(WslOutput::Updated("WSL updated".to_owned())),
    );
}

fn queue_package_update(app: &mut App, terminal: &mut Terminal<WritableTestBackend>) {
    click_label(app, terminal, "F6");
    assert_eq!(app.wsl_requests.pending().len(), 1);
    assert_eq!(
        app.wsl_requests.pending()[0].operation,
        WslOperation::UpdatePackages(
            DistributionName::new("Debian").expect("selected distribution")
        )
    );
    let active = app.wsl_requests.pending()[0].id;
    click_label(app, terminal, "F6");
    assert_eq!(
        app.wsl_requests.pending().len(),
        1,
        "busy click must not queue another update"
    );
    assert_eq!(app.wsl_requests.pending()[0].id, active);
}

fn click_panel_behind_dialog(app: &mut App, terminal: &mut Terminal<WritableTestBackend>) {
    let area = terminal.backend().buffer().area;
    assert!(matches!(
        crate::mouse::hit(
            area,
            70,
            3,
            crate::ui::FKEY_COUNT,
            crate::models::PanelsShown::Both
        ),
        Target::PanelRow {
            side: PanelSide::Right,
            ..
        }
    ));
    dispatch_mouse(
        terminal,
        app,
        mouse(MouseEventKind::Down(MouseButton::Left), 70, 3),
        area,
        &mut PanelClicks::default(),
    );
}
