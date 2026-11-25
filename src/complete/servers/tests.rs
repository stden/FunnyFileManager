//! Purpose: Pin what each Language servers field offers, in which order and
//! with which hint, and how an accepted suggestion lands in the field. The
//! Command field is pinned in `tests/commands.rs`, Arguments in
//! `tests/arguments.rs`, Extensions in `tests/extensions.rs`, and accepting in
//! `tests/landing.rs`.

use std::collections::BTreeMap;

use tempfile::TempDir;

use super::*;
use crate::complete::execs;
use crate::config::{InstallRecipe, ServerSettings};

mod arguments;
mod commands;
mod extensions;
mod landing;

/// The names a fake `PATH` directory lists, as the probe scans them.
fn scanned(names: &[&str]) -> (TempDir, Vec<String>) {
    let bin = TempDir::new().expect("bin");
    for name in names {
        let file = bin.path().join(crate::lsp::program_file(name));
        crate::test_support::write_executable(&file, b"program").expect("program");
    }
    let path = execs::join_paths([bin.path()]);
    let programs = execs::scan(path.as_deref());
    (bin, programs)
}

/// The names as the probe scans them, each from a `PATH` directory of its
/// own: names differing only in case, which one directory holds as one file
/// on Windows and macOS.
fn scanned_apart(names: &[&str]) -> (Vec<TempDir>, Vec<String>) {
    let bins: Vec<TempDir> = names.iter().map(|name| scanned(&[name]).0).collect();
    let path = execs::join_paths(bins.iter().map(TempDir::path));
    let programs = execs::scan(path.as_deref());
    (bins, programs)
}

fn texts(candidates: &[Candidate]) -> Vec<&str> {
    candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect()
}

fn hint<'a>(candidates: &'a [Candidate], text: &str) -> Option<&'a str> {
    candidates
        .iter()
        .find(|candidate| candidate.text == text)
        .and_then(|candidate| candidate.hint.as_deref())
}

fn offered(
    field: CompletedField,
    typed: &str,
    draft: &LspSettings,
    programs: &[String],
) -> Vec<Candidate> {
    let vocabulary = Vocabulary {
        catalog: LspSettings::catalog(),
        draft,
        programs,
    };
    candidates(field, &LineEditor::new(typed), &vocabulary)
}

fn file(name: &str) -> String {
    crate::lsp::program_file(name)
}

/// A line with the caret after `before` and `after` following it.
fn line_at(before: &str, after: &str) -> LineEditor {
    let mut line = LineEditor::new("");
    line.set_split(before, after);
    line
}
