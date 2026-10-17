//! Unit tests: the tree panel draws Far's guide lines, keeps the cursor row on
//! screen, and uses its info row for the folder under the cursor or the
//! search in progress. The tree is built from a made-up listing, so nothing
//! here depends on the disk.

use std::path::{Path, PathBuf};

use ratatui::Terminal;

use super::super::{palette, ui};
use crate::models::{App, FolderTree, PanelKind, PanelSide};
use crate::test_support::{buffer_text, WritableTestBackend};

fn lister(dir: &Path) -> Vec<PathBuf> {
    let children: &[&str] = match dir.to_str().unwrap_or_default() {
        "/" => &["/etc", "/home", "/var"],
        "/etc" => &["/etc/ssh"],
        "/home" => &["/home/ann", "/home/bob"],
        _ => &[],
    };
    children.iter().map(PathBuf::from).collect()
}

fn tree_on(path: &str) -> FolderTree {
    let mut tree = FolderTree::rooted_at(PathBuf::from("/"));
    tree.reveal(Path::new(path), &mut lister);
    tree
}

fn draw(app: &mut App) -> (Terminal<WritableTestBackend>, String) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| ui(f, app)).expect("draw");
    let text = buffer_text(&terminal);
    (terminal, text)
}

#[test]
fn the_rows_carry_fars_guide_lines_and_the_info_row_names_the_cursor() {
    let mut app = App::new().expect("test app");
    app.left_panel.kind = PanelKind::Tree(Box::new(tree_on("/home/bob")));
    let (_, screen) = draw(&mut app);
    assert!(screen.contains(" Tree "), "{screen}");
    assert!(screen.contains("├─etc"), "{screen}");
    assert!(screen.contains("├─home"), "{screen}");
    assert!(screen.contains("│ ├─ann"), "{screen}");
    assert!(screen.contains("│ └─bob"), "{screen}");
    assert!(screen.contains("└─var"), "{screen}");
    assert!(screen.contains("/home/bob"), "the info row: {screen}");
}

#[test]
fn only_the_active_tree_shows_a_cursor_bar() {
    let mut app = App::new().expect("test app");
    app.left_panel.kind = PanelKind::Tree(Box::new(tree_on("/home/bob")));
    let (terminal, screen) = draw(&mut app);
    let row = screen
        .lines()
        .position(|line| line.contains("└─bob"))
        .expect("the cursor row");
    let cell = &terminal.backend().buffer()[(4, u16::try_from(row).expect("row"))];
    assert_eq!(cell.bg, palette::CURSOR_BG, "active: the cursor bar");

    app.active_panel = PanelSide::Right;
    let (terminal, _) = draw(&mut app);
    let cell = &terminal.backend().buffer()[(4, u16::try_from(row).expect("row"))];
    assert_ne!(
        cell.bg,
        palette::CURSOR_BG,
        "inactive: no cursor, as Far draws it"
    );
}

#[test]
fn a_search_in_progress_takes_the_info_row() {
    let mut app = App::new().expect("test app");
    let mut tree = tree_on("/home/bob");
    tree.search.push('b');
    tree.search.push('o');
    app.left_panel.kind = PanelKind::Tree(Box::new(tree));
    let (_, screen) = draw(&mut app);
    assert!(screen.contains("Search: bo_"), "{screen}");
}

#[test]
fn a_long_tree_scrolls_so_the_cursor_row_stays_on_screen() {
    let mut app = App::new().expect("test app");
    let many: Vec<PathBuf> = (0..60)
        .map(|n| PathBuf::from(format!("/dir{n:02}")))
        .collect();
    let mut wide = |dir: &Path| -> Vec<PathBuf> {
        if dir == Path::new("/") {
            many.clone()
        } else {
            Vec::new()
        }
    };
    let mut tree = FolderTree::rooted_at(PathBuf::from("/"));
    tree.expand(&mut wide);
    tree.last();
    app.left_panel.kind = PanelKind::Tree(Box::new(tree));
    let (_, screen) = draw(&mut app);
    assert!(screen.contains("└─dir59"), "{screen}");
    assert!(
        !screen.contains("├─dir00"),
        "scrolled off the top: {screen}"
    );
    let PanelKind::Tree(tree) = &app.left_panel.kind else {
        panic!("still a tree");
    };
    assert!(tree.scroll > 0, "the offset is kept for the next frame");
}

#[test]
fn an_empty_tree_draws_its_frame_and_nothing_else() {
    let mut app = App::new().expect("test app");
    app.left_panel.kind = PanelKind::Tree(Box::default());
    let (_, screen) = draw(&mut app);
    assert!(screen.contains(" Tree "), "{screen}");
    assert!(!screen.contains("├─"), "{screen}");
}
