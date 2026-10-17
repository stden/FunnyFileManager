//! Rendering tests for the unlocked vault browser: the list and tree views,
//! [`MASK`] on every entry password, the reveal and edit dialogs, column
//! widths, the colours and the full-screen frame.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, Terminal};

use super::list::{Columns, COLUMN_GAP, MARKER_WIDTH, MASK, PASSWORD_WIDTH};
use super::render_kdbx;
use super::tests::{draw, draw_at};
use crate::kdbx::tests_support::{focus, password_for, unlocked_sample, SAMPLE};
use crate::kdbx::KdbxSession;
use crate::ui::Cells;

#[test]
fn unlocked_vault_opens_as_a_searchable_tree() {
    let (_dir, session, _now) = unlocked_sample();

    let screen = draw(&session);

    assert!(screen.contains("Tree"), "{screen}");
    assert!(screen.contains("Work"), "{screen}");
    assert!(screen.contains("GitHub"), "{screen}");
    assert!(screen.contains("Filter: _"), "{screen}");
}

#[test]
fn unlocked_list_masks_every_password() {
    let (_dir, mut session, _now) = unlocked_sample();
    session.vault_mut().unwrap().toggle_structure();

    let screen = draw(&session);

    assert!(screen.contains("5 of 5 entries"), "{screen}");
    assert!(screen.contains(MASK));
    for fixture in SAMPLE {
        assert!(
            !screen.contains(fixture.password),
            "{} leaked its password to the screen",
            fixture.title
        );
    }
}

#[test]
fn reveal_draws_the_password_in_a_dedicated_dialog() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.reveal(now).expect("reveal");

    let screen = draw(&session);

    assert!(screen.contains(password_for("GitHub")), "{screen}");
    assert!(screen.contains("Password:"), "{screen}");
    assert!(screen.contains("Enter Copy"), "{screen}");
}

#[test]
fn edit_dialog_shows_the_password_and_full_database_path() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("edit");
    let expected_path = session.path.display().to_string();

    let screen = draw_at(&session, 180, 30);

    assert!(screen.contains(password_for("GitHub")), "{screen}");
    assert!(screen.contains(&expected_path), "{screen}");
    assert!(screen.contains("Ctrl+C Copy password"), "{screen}");
}

#[test]
fn columns_never_exceed_the_width_they_were_fitted_to() {
    // Below this floor the fixed columns alone overflow and the body is
    // clamped to its 3-cell minimum; everything wider must fit exactly.
    let gaps = COLUMN_GAP + COLUMN_GAP + COLUMN_GAP;
    let floor = MARKER_WIDTH + PASSWORD_WIDTH + gaps + Cells::new(3);
    for width in [0, 10, floor.get(), 60, 80, 200].map(Cells::new) {
        let columns = Columns::fit(width);
        let used =
            MARKER_WIDTH + columns.title + columns.username + columns.group + PASSWORD_WIDTH + gaps;
        assert!(used <= width.max(floor), "fit({width}) used {used}");
    }
}

#[test]
fn the_full_ui_dispatches_the_kdbx_overlay() {
    let mut app = crate::models::App::new().expect("test app");
    app.set_overlay(crate::models::Overlay::Kdbx(crate::models::KdbxOverlay(
        KdbxSession::new(PathBuf::from("vault.kdbx")),
    )));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, &mut app)).expect("draw");
    let buf = terminal.backend().buffer();
    let screen: String = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let full = std::env::current_dir().unwrap().join("vault.kdbx");
    assert!(
        screen.contains(&format!("KeePass: {}", full.display())),
        "{screen}"
    );
}

#[test]
fn unlocked_menu_has_white_frame_teal_ground_and_black_selection() {
    let (_dir, session, _now) = unlocked_sample();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| render_kdbx(frame, &session, &mut crate::mouse::DialogMouse::default()))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    let frame = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "╔")
        .expect("frame");
    assert_eq!(frame.bg, crate::ui::palette::MENU_BG);
    assert_eq!(frame.fg, crate::ui::palette::DRIVE_FG);
    let selected = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "►")
        .expect("selection");
    assert_eq!(selected.bg, crate::ui::palette::MENU_SELECTED_BG);
    assert_eq!(selected.fg, crate::ui::palette::DRIVE_FG);
}

#[test]
fn unlocked_vault_uses_the_full_terminal_like_the_file_editor() {
    let (_dir, session, _now) = unlocked_sample();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| render_kdbx(frame, &session, &mut crate::mouse::DialogMouse::default()))
        .expect("draw");
    let buffer = terminal.backend().buffer();

    assert_eq!(buffer[(0, 0)].symbol(), "╔");
    assert_eq!(buffer[(79, 23)].symbol(), "╝");
}

#[test]
fn the_vault_selection_bar_extends_through_the_empty_password_padding() {
    let (_dir, mut session, _now) = unlocked_sample();
    session.vault_mut().unwrap().toggle_structure();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| render_kdbx(frame, &session, &mut crate::mouse::DialogMouse::default()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let body = super::far_block().inner(buffer.area);
    let selected_y = (body.y..body.bottom())
        .find(|&y| buffer[(body.x, y)].symbol() == "►")
        .unwrap();
    assert_eq!(
        buffer[(body.right() - 1, selected_y)].bg,
        crate::ui::palette::MENU_SELECTED_BG
    );
}
