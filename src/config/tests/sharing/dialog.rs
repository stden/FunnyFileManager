//! Unit tests: after the Language servers dialog saves, the exit save writes
//! nothing of its own over the file — neither the session's value nor the
//! whole catalog.

use crate::config::{patch, Enablement, LspSettings};

use super::*;

/// What the Language servers dialog does on F2 for a session that loaded
/// `session`: write `change` of its servers into `path`, and adopt what the
/// file then holds.
fn dialog_save(
    session: &mut Settings,
    path: &Path,
    change: impl FnOnce(&mut LspSettings),
) -> io::Result<()> {
    let mut draft = session.lsp.clone();
    change(&mut draft);
    let edit =
        patch::Edit::new(&session.lsp, &draft).ok_or_else(|| io::Error::other("no change"))?;
    let saved = edit.save(path)?;
    session.adopt_saved_lsp(saved);
    Ok(())
}

fn python(servers: &mut LspSettings) -> &mut crate::config::ServerSettings {
    servers
        .0
        .get_mut("python")
        .expect("python is in the catalog")
}

/// After a dialog save the session holds the servers the file holds, so the
/// exit auto-save has nothing of `lsp` to write — and nothing else either.
#[test]
fn the_exit_save_writes_nothing_after_a_dialog_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).enabled = Enablement::Disabled;
    })?;
    let written = fs::read_to_string(&path)?;
    assert!(
        written.ends_with("system:\n  history_size: 5\nlsp:\n  python:\n    enabled: false\n"),
        "{written}"
    );
    session.save_to(&path)?;
    assert_eq!(fs::read_to_string(&path)?, written);
    Ok(())
}

/// A hand edit of the field the dialog just saved is the user's latest word:
/// the exit save neither refuses it as a conflict nor writes the session's
/// value over it.
#[test]
fn a_hand_edit_after_a_dialog_save_survives_the_exit_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).command = "dialog-pylsp".to_owned();
    })?;
    write_file(&path, "lsp:\n  python:\n    command: hand-pylsp\n")?;
    session.panel.show_hidden = !session.panel.show_hidden;
    session.save_to(&path)?;
    let saved = Settings::load_from(&path)?;
    assert_eq!(saved.lsp.0["python"].command, "hand-pylsp");
    assert_eq!(saved.panel.show_hidden, session.panel.show_hidden);
    Ok(())
}

/// The user undoes a dialog save by removing `lsp`: the exit save keeps the
/// file without it rather than pinning every catalog server into it.
#[test]
fn the_exit_save_never_pins_the_catalog_after_a_dialog_save() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut session = Settings::load_from(&path)?;
    dialog_save(&mut session, &path, |servers| {
        python(servers).enabled = Enablement::Disabled;
    })?;
    write_file(&path, "system:\n  history_size: 5\n")?;
    session.panel.show_hidden = !session.panel.show_hidden;
    session.save_to(&path)?;
    let text = fs::read_to_string(&path)?;
    assert!(!text.contains("lsp"), "{text}");
    Ok(())
}
