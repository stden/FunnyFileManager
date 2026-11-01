//! Purpose: Publish a bounded view of the application without serializing secret-bearing models.
use crate::{
    application::{automation::current_context, headless::Runtime},
    models::{App, Overlay, PanelSide},
};
use serde_json::{json, Value};

mod image;
mod modal;
mod panels;
mod pending;

pub(super) fn is_pending(app: &App) -> bool {
    pending::is_pending(app)
}

pub(super) fn capture(runtime: &Runtime) -> Value {
    let app = &runtime.app;
    let buffer = runtime.terminal.backend().buffer();
    let redacted = secret_visible(&app.overlay);
    let rows: Vec<String> = if redacted {
        vec!["[Revealed secret omitted from passive screen state]".to_owned()]
    } else {
        buffer
            .content
            .chunks(usize::from(buffer.area.width).max(1))
            .map(|row| {
                row.iter()
                    .map(ratatui::buffer::Cell::symbol)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    };
    json!({
        "running": app.lifecycle.is_running(),
        "panels": {"left": panels::describe(&app.left_panel), "right": panels::describe(&app.right_panel),
            "active": match app.active_panel { PanelSide::Left => "left", PanelSide::Right => "right" }},
        "modal": {"kind": modal::kind(&app.overlay), "context": current_context(app).map(crate::keymap::Context::name)},
        "screen": {"width": buffer.area.width, "height": buffer.area.height, "rows": rows, "redacted": redacted},
        "status": app.status_text(), "editor": editor(&app.overlay),
        "image": image::viewer(&app.overlay),
        "background_pending": pending::is_pending(app)
    })
}

fn editor(overlay: &Overlay) -> Value {
    let Overlay::Editor(editor) = overlay else {
        return Value::Null;
    };
    json!({"title": editor.title, "path": editor.path.to_string_lossy(), "modified": editor.state.is_modified(),
        "cursor_line": editor.cursor_line, "cursor_column": editor.cursor_col, "line_count": editor.lines.len()})
}

#[cfg(feature = "keepass")]
fn secret_visible(overlay: &Overlay) -> bool {
    let Overlay::Kdbx(session) = overlay else {
        return false;
    };
    session
        .0
        .prompt()
        .is_some_and(crate::kdbx::UnlockPrompt::is_revealed)
        || session
            .0
            .vault()
            .is_some_and(crate::kdbx::Vault::is_revealed)
}
#[cfg(not(feature = "keepass"))]
const fn secret_visible(_overlay: &Overlay) -> bool {
    false
}
