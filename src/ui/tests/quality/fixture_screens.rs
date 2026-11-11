//! Panel modes, long Unicode text and full-screen readers.

use std::path::PathBuf;

use crate::{
    menu::{Heading, Menu},
    models::{
        ImageState, Overlay, PanelKind, PanelView, Preview, QuickView, SessionsPanel, TextPreview,
        ViewMode, Viewer,
    },
    sessions::SessionStore,
};

use super::{editor, overlay, plain, viewer, Case};

pub(super) fn cases() -> Vec<Case> {
    let mut cases = vec![
        plain("panels", &["selected"]),
        overlay("help", Overlay::Help, &["Help", "F1:"]),
        overlay("tasks", Overlay::Tasks(Box::default()), &["Task list"]),
        overlay(
            "information",
            Overlay::Information(Box::default()),
            &["Information"],
        ),
    ];
    cases.extend(readers());
    cases.extend(diagnostics());
    cases.extend(panel_modes());
    cases.extend(image_states());
    cases.extend(menus());
    cases.extend(panel_views());
    cases
}

fn readers() -> Vec<Case> {
    vec![
        overlay(
            "viewer",
            Overlay::Viewer(viewer()),
            &["Preview", "Проверка"],
        ),
        overlay(
            "editor",
            Overlay::Editor(editor()),
            &["quality.rs", "fn main"],
        ),
    ]
}

fn menus() -> Vec<Case> {
    Heading::ALL
        .into_iter()
        .map(|heading| {
            let first = match heading {
                Heading::Left | Heading::Right => "Brief",
                Heading::Files => "View",
                Heading::Commands => "Find file",
                Heading::Options => "System settings",
            };
            overlay(
                heading.label(),
                Overlay::Menu(Menu::open(heading)),
                &[first],
            )
        })
        .collect()
}

fn panel_views() -> Vec<Case> {
    PanelView::ALL
        .into_iter()
        .map(|view| {
            let mut case = plain(view.label(), &["sele"]);
            case.app.left_panel.view = view;
            case.app.left_panel.details.descriptions.insert(
                "selected.txt".to_owned(),
                "Readable description Проверка 界".to_owned(),
            );
            case.app
                .left_panel
                .details
                .owners
                .insert("selected.txt".to_owned(), "operator".to_owned());
            case.app
                .left_panel
                .details
                .links
                .insert("selected.txt".to_owned(), "target".to_owned());
            case
        })
        .collect()
}

fn panel_modes() -> Vec<Case> {
    let mut empty = plain("empty", &["No visible entries"]);
    empty.wide_required = vec!["F7"];
    empty.app.left_panel.entries.clear();
    let mut filtered = plain("filtered", &["No matches", "Filter"]);
    filtered.app.left_panel.entries.clear();
    filtered.app.left_panel.filter.hide_dotfiles = true;
    let mut netbox = plain("netbox", &["sessions"]);
    netbox.wide_required = vec!["NetBox", "Session name"];
    netbox.app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(
        &SessionStore::default(),
    )));
    let mut console = plain("console", &["Command output"]);
    console.app.view = ViewMode::Console;
    console.app.console.record_output("Command output");
    let mut tree = plain("tree", &["Tree"]);
    tree.app.left_panel.kind = PanelKind::Tree(Box::default());
    vec![empty, filtered, netbox, quick_view(), console, tree]
}

fn quick_view() -> Case {
    let mut case = plain("quick-view", &["UTF-8", "Проверка"]);
    case.app.right_panel.kind = PanelKind::QuickView(Box::new(QuickView {
        shown: Some(PathBuf::from("/ffm-quality/preview.txt")),
        preview: Preview::Text(TextPreview::decoded(
            std::path::Path::new("/ffm-quality/preview.txt"),
            "Проверка 界 e\u{301}".as_bytes(),
        )),
        ..QuickView::default()
    }));
    case
}

fn image_states() -> Vec<Case> {
    let pending = Viewer::image(PathBuf::from("/ffm-quality/photo.png"));
    let mut failed = pending.clone();
    failed.image = Some(ImageState::Failed("Invalid PNG data".to_owned()));
    vec![
        overlay(
            "image-loading",
            Overlay::Viewer(pending),
            &["Loading image"],
        ),
        overlay(
            "image-error",
            Overlay::Viewer(failed),
            &["Cannot show image", "Esc"],
        ),
    ]
}

fn diagnostics() -> Vec<Case> {
    [
        ("editor-error", 1),
        ("editor-warning", 2),
        ("editor-hint", 4),
    ]
    .into_iter()
    .map(|(name, severity)| {
        let mut editor = editor();
        editor.lsp.state = crate::models::ServerState::Ready;
        editor.lsp.diagnostics.push(diagnostic(severity));
        overlay(name, Overlay::Editor(editor), &["fn main"])
    })
    .collect()
}

fn diagnostic(severity: u32) -> crate::lsp::features::Diagnostic {
    use crate::lsp::positions::{Position, Range};

    crate::lsp::features::Diagnostic {
        range: Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 2,
            },
        },
        message: "Readable diagnostic".to_owned(),
        severity: Some(severity),
    }
}
