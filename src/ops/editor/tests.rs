//! Unit tests: `Editor::save` when the target file does not exist yet, and
//! when the path the editor was handed is not the file itself.

use std::{fs, io};

use tempfile::TempDir;

use crate::models::*;

fn dir_names(dir: &std::path::Path) -> io::Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)?
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    Ok(names)
}

struct OscClipboard;

impl crate::clipboard::ClipboardBackend for OscClipboard {
    fn get_native(&mut self) -> Result<String, String> {
        Err("no native clipboard".to_owned())
    }
    fn set_native(&mut self, _: &str) -> Result<(), String> {
        Err("no native clipboard".to_owned())
    }
    fn emit_osc52(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

mod clipboard_fallback;
mod dialog_state;
mod save_targets;
mod save_writes;
mod write_permissions;
