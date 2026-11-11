//! Purpose: The overlays drawn from the menu-dialog path, each as its own
//! question: an existing-file question names the clash it asks about.

use ratatui::Terminal;
use tempfile::TempDir;

use crate::copy::CopyPrompt;
use crate::models::{App, Clash, ExistingFile, Overlay, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

#[test]
fn an_existing_file_question_is_drawn_with_the_name_it_clashes_on() {
    let root = TempDir::new().expect("temporary directory");
    let source = root.path().join("report.txt");
    let dest = root.path().join("dest").join("report.txt");
    std::fs::create_dir_all(dest.parent().expect("parent")).expect("dest folder");
    std::fs::write(&source, b"new").expect("source");
    std::fs::write(&dest, b"old").expect("dest");
    let clash = Clash::between(&source, &dest);
    let settings = CopyPrompt::new("report.txt", root.path());
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("panel"),
        Panel::new(root.path().to_path_buf()).expect("panel"),
    );
    app.overlay = Overlay::ExistingFile(Box::new(ExistingFile::new(
        settings,
        vec![source],
        root.path().to_path_buf(),
        vec![clash],
    )));
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");

    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");

    assert!(buffer_text(&terminal).contains("report.txt"));
}
