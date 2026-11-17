//! Purpose: A fake machine for the Language servers dialog's tests — a
//! settings file, an LSP cache and a `PATH` directory, all in one `TempDir` —
//! shared by the ops, popup and rendering tests, with the keys they press.

use std::{fs, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::config::LanguageId;
use crate::models::{settings::Settings, App, LspEditor, Overlay, Panel};

/// `bin/` (the whole `PATH`), `cache/` (the LSP cache), `work/` (both panels)
/// and `config/settings.yaml`, which does not exist until written.
pub struct LspMachine {
    pub home: TempDir,
}

impl LspMachine {
    pub fn new() -> Self {
        let home = TempDir::new().expect("home");
        for dir in ["bin", "cache", "work", "config"] {
            fs::create_dir(home.path().join(dir)).expect("dir");
        }
        Self { home }
    }

    pub fn settings(&self) -> PathBuf {
        self.home
            .path()
            .join("config")
            .join(crate::config::document::FILE_NAME)
    }

    pub fn bin(&self) -> PathBuf {
        self.home.path().join("bin")
    }

    pub fn write_settings(&self, text: &str) {
        fs::write(self.settings(), text).expect("settings");
    }

    /// The settings file as YAML: compared as values, because a save writes
    /// its header comment first and quotes as it likes.
    pub fn parsed(&self) -> serde_norway::Value {
        let text = fs::read_to_string(self.settings()).expect("settings file");
        serde_norway::from_str(&text).expect("YAML")
    }

    /// An app on this machine, the settings file injected.
    pub fn app(&self) -> App {
        let panel = Panel::from_entries(self.home.path().join("work"), Vec::new());
        let mut app = App::with_panels(panel.clone(), panel);
        app.settings = Settings::with_file(self.settings());
        super::set_lsp_probe(
            &mut app,
            &self.home.path().join("cache"),
            &[self.bin().as_path()],
        );
        app
    }

    /// Put what the catalog's recipes install with on this machine's `PATH` —
    /// Python 3, Java, and Node.js with npm beside it — so a recipe that is
    /// not installed yet reads `auto`, not `error`.
    pub fn install_runtimes(&self) {
        for name in ["python3", "java", "node"] {
            let program = self.bin().join(crate::lsp::program_file(name));
            super::write_executable(&program, b"runtime").expect("runtime");
        }
        let npm = self.bin().join("node_modules/npm/bin/npm-cli.js");
        fs::create_dir_all(npm.parent().expect("parent")).expect("npm dir");
        fs::write(npm, "npm").expect("npm");
    }

    /// The app with the dialog open.
    pub fn opened(&self) -> App {
        let mut app = self.app();
        crate::ops::lsp_editor::open(&mut app);
        app
    }
}

/// The open dialog.
pub fn editor(app: &App) -> &LspEditor {
    let Overlay::LspEditor(editor) = &app.overlay else {
        panic!("the Language servers dialog is open")
    };
    editor
}

/// The open dialog, to change.
pub fn editor_mut(app: &mut App) -> &mut LspEditor {
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        panic!("the Language servers dialog is open")
    };
    editor
}

/// Press `code` in the open dialog, as the event loop hands it over.
pub fn press(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::NONE);
}

/// Press `code` with `modifiers` held.
pub fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let taken = crate::popup::process_popup_input(app, KeyEvent::new(code, modifiers));
    assert!(taken, "an open dialog takes every key");
}

/// Type `text` into the focused field, one key per character.
pub fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

/// Select `language`'s row.
pub fn select(app: &mut App, language: &str) {
    editor_mut(app).select(&LanguageId::from_key(language));
}
