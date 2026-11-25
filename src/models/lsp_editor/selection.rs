//! Purpose: The selected server of the Language servers dialog: moving it,
//! keeping it across removals and saves, and Space and Del on it.

use crate::config::{patch::with_owned_fields, LanguageId, LspSettings, ServerSettings};

use super::{LspEditor, Origin, Step};

impl LspEditor {
    #[must_use]
    pub const fn selected(&self) -> Option<&LanguageId> {
        self.selected.as_ref()
    }

    /// The selected row's position in the list.
    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.draft.0.keys().position(|key| key == selected.as_str())
    }

    /// The selected server and its language.
    #[must_use]
    pub fn selected_server(&self) -> Option<(&LanguageId, &ServerSettings)> {
        let selected = self.selected.as_ref()?;
        self.draft
            .0
            .get(selected.as_str())
            .map(|server| (selected, server))
    }

    pub fn select(&mut self, language: &LanguageId) {
        if self.draft.0.contains_key(language.as_str()) {
            self.selected = Some(language.clone());
        }
    }

    /// Move the selection, clamped to the list.
    pub fn move_selection(&mut self, step: Step) {
        let last = self.draft.0.len().saturating_sub(1);
        let Some(current) = self.selected_index() else {
            return;
        };
        let page = self.drawn.list_page();
        let next = match step {
            Step::Up => current.saturating_sub(1),
            Step::Down => current.saturating_add(1).min(last),
            Step::PageUp => current.saturating_sub(page),
            Step::PageDown => current.saturating_add(page).min(last),
            Step::First => 0,
            Step::Last => last,
        };
        self.select_index(next);
    }

    /// Select the row at `index`, clamped to the table; nothing when empty.
    fn select_index(&mut self, index: usize) {
        let last = self.draft.0.len().saturating_sub(1);
        self.selected = self
            .draft
            .0
            .keys()
            .nth(index.min(last))
            .map(|key| LanguageId::from_key(key));
    }

    /// Keep the selected language when it is still in the table, otherwise
    /// select the row now at its old position.
    fn reselect(&mut self, old_index: Option<usize>) {
        let still_there = self
            .selected
            .as_ref()
            .is_some_and(|language| self.draft.0.contains_key(language.as_str()));
        if !still_there {
            self.select_index(old_index.unwrap_or(0));
        }
    }

    /// Space: flip the selected server on or off, in the draft.
    pub fn toggle_selected(&mut self) -> Option<LanguageId> {
        let language = self.selected.clone()?;
        let server = self.draft.0.get_mut(language.as_str())?;
        server.enabled = server.enabled.toggled();
        Some(language)
    }

    /// Del: put a catalog server's four owned fields back to the catalog, or
    /// remove an added server, as its [`Origin`] says. Only the draft changes.
    pub fn delete_selected(&mut self) -> Option<(LanguageId, Origin)> {
        let language = self.selected.clone()?;
        let old_index = self.selected_index();
        let origin = Self::origin(&language);
        if let Some(catalog) = LspSettings::catalog().0.get(language.as_str()) {
            let server = self.draft.0.get_mut(language.as_str())?;
            *server = with_owned_fields(server.clone(), catalog);
        } else {
            self.draft.0.remove(language.as_str());
            self.availability.remove(&language);
            self.reselect(old_index);
        }
        Some((language, origin))
    }

    /// After a save: what the file now holds is both the baseline and the
    /// draft, and the selection stays on its language when it is still there.
    pub(crate) fn saved(&mut self, settings: LspSettings) {
        let old_index = self.selected_index();
        self.baseline = settings.clone();
        self.draft = settings;
        self.reselect(old_index);
    }
}
