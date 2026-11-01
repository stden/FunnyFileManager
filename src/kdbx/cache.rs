//! Keys stay in this application's memory; cloning never duplicates them.
use super::{Access, KdbxSession, KdbxState};
use std::{path::PathBuf, time::Instant};

#[derive(Default)]
pub struct SessionCache(Vec<KdbxSession>);

impl Clone for SessionCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl SessionCache {
    pub(crate) fn remember(&mut self, mut session: KdbxSession) {
        let Some(vault) = session.vault_mut() else {
            return;
        };
        vault.hide();
        vault.clear_edit();
        vault.show_tree();
        vault.notice = None;
        self.0.retain(|saved| saved.path != session.path);
        if self.0.len() == 16 {
            self.0.remove(0);
        }
        self.0.push(session);
    }

    pub(crate) fn take(&mut self, path: PathBuf, access: Access, now: Instant) -> KdbxSession {
        self.expire(now);
        let mut session = self
            .0
            .iter()
            .position(|saved| saved.path == path)
            .map_or_else(|| KdbxSession::new(path), |index| self.0.remove(index));
        session.access = access;
        session.refresh(now);
        session
    }

    pub(crate) fn expire(&mut self, now: Instant) {
        for session in &mut self.0 {
            session.tick(now);
        }
        self.0.retain(|session| !session.state.is_locked());
    }
}

impl KdbxSession {
    fn refresh(&mut self, now: Instant) {
        let Some(vault) = self.vault() else {
            return;
        };
        let result =
            crate::keepass::read_catalog(&self.path, vault.master().expose(), vault.keyfile());
        match result {
            Ok(catalog) => {
                if let KdbxState::Unlocked(vault) = &mut self.state {
                    vault.replace_catalog(catalog.entries, catalog.groups, now);
                }
            }
            Err(_) => self.lock(),
        }
    }
}

#[cfg(test)]
mod tests;
