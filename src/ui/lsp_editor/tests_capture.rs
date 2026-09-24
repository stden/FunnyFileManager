//! Purpose: Capture the Language servers dialog for the documentation's
//! screenshot, drawn from synthetic settings and paths, never the machine's.

use ratatui::Terminal;
use serde_json::{json, Value};

use crate::config::{LanguageId, LspSettings};
use crate::lsp::{Availability, NotInstalled, Resolved};
use crate::models::lsp_editor::Access;
use crate::models::{App, LspEditor, Overlay, Panel};
use crate::WritableTestBackend;

/// What the screenshot says each server would start.
fn synthetic(language: &str) -> Availability {
    match language {
        "python" => Availability::Cached(Resolved {
            executable: "/home/bob/.config/ffm/lsp/7c1e/venv/bin/python".into(),
            args: vec!["-m".to_owned(), "pylsp".to_owned()],
        }),
        "bat" => Availability::NotInstalled(NotInstalled::AutoInstallOff),
        "css" | "html" | "json" => Availability::Configured(Resolved {
            executable: format!("/usr/local/bin/vscode-{language}-language-server").into(),
            args: vec!["--stdio".to_owned()],
        }),
        _ => Availability::NotInstalled(NotInstalled::AfterTrust),
    }
}

/// The dialog as a user with a customised python and the batch-file server
/// turned off sees it: the first screen shows a disabled, missing server, and
/// F3's a configured executable.
pub(super) fn review_app() -> App {
    let loaded: LspSettings =
        serde_norway::from_str("python: {args: []}\nbat: {enabled: false}\n").expect("valid");
    let mut editor = LspEditor::new(
        Access::Writable {
            path: "/home/bob/.config/ffm/settings.yaml".into(),
        },
        loaded,
    );
    let found: Vec<(LanguageId, Availability)> = editor
        .draft()
        .0
        .keys()
        .map(|language| (LanguageId::from_key(language), synthetic(language)))
        .collect();
    for (language, availability) in found {
        editor.set_availability(language, availability);
    }
    editor.select(&LanguageId::from_key("python"));
    let panel = || Panel::from_entries("/home/bob/project".into(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.set_overlay(Overlay::LspEditor(Box::new(editor)));
    app
}

#[test]
#[ignore = "explicit review artifact capture with synthetic paths"]
fn capture_lsp_editor_review_screen() {
    let (width, height) = (80, 24);
    let mut app = review_app();
    let mut terminal = Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");
    let cells: Vec<Value> = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| json!({"text": cell.symbol(), "fg": cell.fg.to_string(), "bg": cell.bg.to_string()}))
        .collect();
    assert!(cells.iter().any(|cell| cell["text"] == "▸"));
    println!(
        "FFM_SCREEN:{}",
        json!({"width": width, "height": height, "cells": cells})
    );
}
