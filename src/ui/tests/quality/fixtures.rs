//! Fixed screens used by quality checks; vaults use isolated temporary files.

use std::path::PathBuf;

use ratatui::buffer::Buffer;

use crate::models::{
    App, DirEntry, DirectoryListing, Editor, EntryKind, FileName, FileSize, Overlay, Panel,
    PanelMetadata, Viewer,
};

#[path = "fixture_checks.rs"]
mod checks;
#[path = "fixture_dialogs.rs"]
mod dialogs;
#[path = "fixture_interactions.rs"]
mod interactions;
#[path = "fixture_lsp.rs"]
mod lsp;
#[path = "fixture_screens.rs"]
mod screens;
#[path = "fixture_statuses.rs"]
mod statuses;

pub(super) struct Case {
    pub name: &'static str,
    pub app: App,
    pub required: Vec<&'static str>,
    /// Extra labels offered by the full-width layout, beyond compact hints.
    pub wide_required: Vec<&'static str>,
    /// The application/session drops before its isolated fixture directory.
    #[cfg(feature = "keepass")]
    pub resources: Vec<tempfile::TempDir>,
}

pub(super) fn cases() -> Vec<Case> {
    let mut cases = screens::cases();
    cases.extend(dialogs::cases());
    cases.extend(interactions::cases());
    cases.extend(lsp::cases());
    cases.extend(statuses::cases());
    cases
}

pub(super) fn validate(buffer: &Buffer, case: &Case, width: u16, height: u16) {
    checks::validate(buffer, case, width, height);
}

fn overlay(name: &'static str, overlay: Overlay, required: &[&'static str]) -> Case {
    let mut case = plain(name, required);
    case.app.set_overlay(overlay);
    case
}

fn plain(name: &'static str, required: &[&'static str]) -> Case {
    Case {
        name,
        app: app(),
        required: required.to_vec(),
        wide_required: Vec::new(),
        #[cfg(feature = "keepass")]
        resources: Vec::new(),
    }
}

fn app() -> App {
    let mut left = Panel::from_entries(PathBuf::from("/ffm-quality/left"), Vec::new());
    left.apply_listing(DirectoryListing {
        entries: entries(),
        parent: None,
        metadata: PanelMetadata {
            hidden_names: std::collections::BTreeSet::from([FileName::from(".hidden")]),
            dirty_repos: std::collections::BTreeSet::from([FileName::from("repo")]),
            ..PanelMetadata::default()
        },
    });
    left.marked.insert(FileName::from("marked.txt"));
    left.select_named("selected.txt");
    let right = Panel::from_entries(PathBuf::from("/ffm-quality/right"), entries());
    App::with_panels(left, right)
}

fn entries() -> Vec<DirEntry> {
    [
        ("..", EntryKind::Directory),
        ("repo", EntryKind::Directory),
        ("selected.txt", EntryKind::File),
        ("marked.txt", EntryKind::File),
        (".hidden", EntryKind::File),
        ("run.sh", EntryKind::File),
        ("очень-длинное-имя-файла-界-e\u{301}.txt", EntryKind::File),
    ]
    .into_iter()
    .map(|(name, kind)| DirEntry {
        name: name.to_owned(),
        kind,
        executable: name == "run.sh",
        size: FileSize::from_bytes(4096),
        ..DirEntry::default()
    })
    .collect()
}

fn viewer() -> Viewer {
    Viewer::report("Preview", vec!["Проверка 界 e\u{301}".to_owned()])
}

fn editor() -> Editor {
    Editor::from_content(
        "quality.rs".to_owned(),
        PathBuf::from("/ffm-quality/quality.rs"),
        "// Проверка 界 e\u{301}\nfn main() {\n    println!(\"Readable\");\n}\n",
    )
}
