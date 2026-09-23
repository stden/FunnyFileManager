use crate::{
    models::{App, Overlay},
    path_env::{Mode, PathEditor, PathEntry, Scope, Snapshot, Validation},
};
use std::io;

mod store;
mod validation;
pub use validation::validate;

pub fn open(app: &mut App, scope: Scope) -> io::Result<()> {
    let mut editor = PathEditor::new(scope, store::read(scope)?);
    recheck(&mut editor);
    app.set_overlay(Overlay::PathEditor(Box::new(editor)));
    Ok(())
}

pub fn recheck(editor: &mut PathEditor) {
    for entry in &mut editor.entries {
        entry.validation = validate(&entry.text);
    }
}

pub fn accept_entry(editor: &mut PathEditor) {
    let Mode::Edit { index, input } = &editor.mode else {
        return;
    };
    let text = input.text().trim().trim_matches('"').to_owned();
    if text.contains(crate::path_env::separator()) || text.contains(['\n', '\r', '\0']) {
        "Enter one directory per row".clone_into(&mut editor.message);
        return;
    }
    let entry = PathEntry {
        validation: validate(&text),
        text,
    };
    if let Some(index) = index {
        editor.entries[*index] = entry;
    } else {
        editor.entries.push(entry);
        editor.selected = editor.entries.len() - 1;
    }
    editor.mode = Mode::Browse;
    editor.message.clear();
}

pub fn save(editor: &mut PathEditor) -> io::Result<()> {
    save_with(editor, |scope, previous, value| {
        store::write(scope, previous, value)?;
        store::read(scope)
    })
}

fn save_with(
    editor: &mut PathEditor,
    write: impl FnOnce(Scope, &Snapshot, &str) -> io::Result<Snapshot>,
) -> io::Result<()> {
    recheck(editor);
    if editor
        .entries
        .iter()
        .any(|entry| entry.validation != Validation::Directory)
    {
        return Err(io::Error::other(
            "Fix or remove invalid paths before saving",
        ));
    }
    editor.snapshot = write(editor.scope, &editor.snapshot, &editor.value())?;
    editor.mode = Mode::Browse;
    "PATH saved. Open a new terminal / login session to use it.".clone_into(&mut editor.message);
    Ok(())
}

#[cfg(test)]
mod tests;
