//! Purpose: Change the stored sessions from the panel — the session dialog,
//! a copy, a delete — each through [`change`], so the store in memory and on
//! disk is either wholly changed or left as it was.

use super::{find, refresh, SessionError};
use crate::models::{
    App, EditPurpose, Origin, Overlay, PanelKind, Session, SessionEdit, SessionName,
};
use crate::sessions::SessionStore;
use crate::status::Status;

/// Apply `edit` to a copy of the store, save the copy, and only then make it
/// the store. On any failure the store — in memory and on disk — is as it
/// was.
pub(super) fn change(
    app: &mut App,
    edit: impl FnOnce(&mut SessionStore) -> Result<(), SessionError>,
) -> Result<(), SessionError> {
    let mut store = app.sessions.clone();
    edit(&mut store)?;
    store.save().map_err(SessionError::Store)?;
    app.sessions = store;
    refresh(app);
    Ok(())
}

/// Put the cursor of the active sessions panel on `name`.
fn focus(app: &mut App, name: &SessionName) {
    if let PanelKind::Sessions(panel) = &mut app.get_active_panel().kind {
        panel.focus(name);
    }
}

/// `Shift+F4`: the empty session dialog.
pub fn create(app: &mut App) {
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
}

/// `F4`: the dialog filled from the session `name`.
pub fn edit(app: &mut App, name: &SessionName) -> Result<(), SessionError> {
    let session = find(app, name)?;
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::from_session(
        session,
    ))));
    Ok(())
}

/// Ok in the session dialog: store what it describes. The dialog stays open
/// on an error, so a mistyped port is corrected rather than retyped.
pub fn save_edit(app: &mut App, dialog: &SessionEdit) -> Result<(), SessionError> {
    let session = dialog.to_session()?;
    let name = session.name.clone();
    let replaced = match dialog.purpose() {
        EditPurpose::Create => None,
        EditPurpose::Edit(old) => Some(old.clone()),
    };
    change(app, |store| {
        if replaced.as_ref() != Some(&name) && store.find(&name).is_some() {
            return Err(SessionError::NameTaken(name.clone()));
        }
        if let Some(old) = &replaced {
            store.remove(old);
        }
        store.upsert(session);
        Ok(())
    })?;
    app.close_overlay();
    focus(app, &name);
    app.set_status(Status::CommandOutput(format!("NetBox: saved {name}")));
    Ok(())
}

/// F5 once named: the session `name` again as `copy`, the user's own
/// whatever the original was imported from.
pub fn duplicate(app: &mut App, name: &SessionName, copy: &str) -> Result<(), SessionError> {
    let copy = SessionName::new(copy)?;
    let original = find(app, name)?.clone();
    change(app, |store| {
        if store.find(&copy).is_some() {
            return Err(SessionError::NameTaken(copy.clone()));
        }
        store.upsert(Session {
            name: copy.clone(),
            origin: Origin::Manual,
            ..original
        });
        Ok(())
    })?;
    focus(app, &copy);
    app.set_status(Status::CommandOutput(format!(
        "NetBox: copied {name} to {copy}"
    )));
    Ok(())
}

/// F8 once confirmed.
pub fn delete(app: &mut App, name: &SessionName) -> Result<(), SessionError> {
    change(app, |store| {
        store
            .remove(name)
            .map(drop)
            .ok_or_else(|| SessionError::NotFound(name.clone()))
    })?;
    app.set_status(Status::CommandOutput(format!("NetBox: deleted {name}")));
    Ok(())
}
