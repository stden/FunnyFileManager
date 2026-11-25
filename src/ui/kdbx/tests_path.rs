//! Purpose: A relative vault path is shown under the working folder, so the
//! title names the full file.

use std::path::{Path, PathBuf};

use super::*;

#[test]
fn a_relative_vault_path_is_shown_under_the_working_folder() {
    let shown = display_path(Path::new("vault.kdbx"));
    assert!(shown.ends_with("vault.kdbx"), "{shown}");
    assert!(shown.len() > "vault.kdbx".len(), "{shown}");
}

#[test]
fn a_long_absolute_locked_title_keeps_keepass_and_the_file_name() {
    for name in ["vault.kdbx", "хранилище界.kdbx"] {
        let path = absolute_path(&format!(
            "{}{name}",
            "a-long-workspace-directory/".repeat(8)
        ));
        assert!(path.is_absolute(), "the fixture must not depend on the cwd");
        let session = KdbxSession::new(path);
        for width in [40, 80, 120] {
            let screen = draw_without_continuations(&session, width);
            assert!(screen.contains("KeePass:"), "{width}: {screen}");
            assert!(screen.contains(name), "{width}: {screen}");
            assert!(
                screen.contains('…'),
                "the shortened path must be marked: {screen}"
            );
        }
    }
}

#[test]
fn a_fitting_absolute_locked_title_keeps_the_full_path() {
    let path = absolute_path("vaults/private/vault.kdbx");
    let shown = display_path(&path);
    let session = KdbxSession::new(path);
    let screen = tests::draw_at(&session, 120, 24);
    assert!(screen.contains(&format!("KeePass: {shown}")), "{screen}");
}

fn absolute_path(tail: &str) -> PathBuf {
    let root = if cfg!(windows) { "C:/" } else { "/" };
    PathBuf::from(format!("{root}{tail}"))
}

fn draw_without_continuations(session: &KdbxSession, width: u16) -> String {
    use ratatui::{backend::TestBackend, Terminal};

    let mut terminal = Terminal::new(TestBackend::new(width, 12)).unwrap();
    terminal
        .draw(|frame| {
            render_kdbx(frame, session, &mut crate::mouse::DialogMouse::default());
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(usize::from(width))
        .map(visible_row)
        .collect::<Vec<_>>()
        .join("\n")
}

fn visible_row(cells: &[ratatui::buffer::Cell]) -> String {
    let mut text = String::new();
    let mut continuation = 0;
    for cell in cells {
        if continuation > 0 {
            assert!(cell.symbol().trim().is_empty(), "wide glyph overlaps text");
            continuation -= 1;
            continue;
        }
        text.push_str(cell.symbol());
        continuation = Cells::of(cell.symbol()).get().saturating_sub(1);
    }
    text
}
