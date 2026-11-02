//! Purpose: The Language servers dialog's state — the servers loaded from
//! `settings.yaml`, the draft being edited, the selection, and the mode.
//!
//! Private fields with accessors for `ui` and mutations for `ops` and
//! `popup`. Nothing here reads a file or the machine: availability arrives
//! from `ops`, and the draft reaches the file only through a save there.

use std::collections::BTreeMap;

use crate::config::{LanguageId, LspSettings};
use crate::lsp::Availability;
use crate::models::ExitChoice;
use crate::scroll::ScrollTop;

mod access;
mod form;
mod form_mode;
mod paging;
mod problems;
mod selection;
pub use access::{Access, ReadOnlyReason};
pub use form::{Accepted, Direction, FormField, FormTarget, ListFocus, ServerForm, Suggestions};
pub use paging::{Drawn, Step};
pub use problems::{problems, Impact, Problem};

/// What the dialog shows and what its keys do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// The server list with the selected server's detail.
    Browse,
    /// F3: every server, scrolled.
    View(ScrollTop),
    Form(Box<ServerForm>),
    /// Esc with unsaved changes: save, discard, or stay.
    ConfirmExit(ExitChoice),
}

/// The message row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Info(String),
    Error(String),
}

/// Where a server comes from, which decides whether Del resets or removes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Catalog,
    Added,
}

/// The Language servers dialog.
#[derive(Debug, Clone)]
pub struct LspEditor {
    access: Access,
    /// What the file held at open or at the last save, over the catalog.
    baseline: LspSettings,
    draft: LspSettings,
    availability: BTreeMap<LanguageId, Availability>,
    /// `None` only for an empty table.
    selected: Option<LanguageId>,
    mode: Mode,
    notice: Option<Notice>,
    drawn: Drawn,
}

impl LspEditor {
    /// The dialog over `loaded`, on its first server.
    #[must_use]
    pub(crate) fn new(access: Access, loaded: LspSettings) -> Self {
        let selected = loaded.0.keys().next().map(|key| LanguageId::from_key(key));
        Self {
            access,
            baseline: loaded.clone(),
            draft: loaded,
            availability: BTreeMap::new(),
            selected,
            mode: Mode::Browse,
            notice: None,
            drawn: Drawn::default(),
        }
    }

    #[must_use]
    pub(crate) const fn access(&self) -> &Access {
        &self.access
    }

    #[must_use]
    pub const fn draft(&self) -> &LspSettings {
        &self.draft
    }

    #[must_use]
    pub const fn baseline(&self) -> &LspSettings {
        &self.baseline
    }

    #[must_use]
    pub const fn mode(&self) -> &Mode {
        &self.mode
    }

    pub const fn mode_mut(&mut self) -> &mut Mode {
        &mut self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// The open form, while the dialog is in form mode.
    pub fn form_mut(&mut self) -> Option<&mut ServerForm> {
        match &mut self.mode {
            Mode::Form(form) => Some(form),
            Mode::Browse | Mode::View(_) | Mode::ConfirmExit(_) => None,
        }
    }

    #[must_use]
    pub const fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    pub fn set_notice(&mut self, notice: Notice) {
        self.notice = Some(notice);
    }

    /// Hand the message row back to what changed, or to the read-only reason.
    pub fn clear_notice(&mut self) {
        self.notice = None;
    }

    /// Why a read-only dialog changes nothing, as its message row says it;
    /// `None` when the dialog may change the table and save.
    #[must_use]
    pub fn read_only_notice(&self) -> Option<Notice> {
        match &self.access {
            Access::ReadOnly(reason) => Some(Notice::Error(format!("Read only: {reason}."))),
            Access::Writable { .. } => None,
        }
    }

    /// Whether the draft differs from what the file holds.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.draft != self.baseline
    }

    /// Whether `language` differs from what the file holds: the `*` mark.
    #[must_use]
    pub fn is_changed(&self, language: &LanguageId) -> bool {
        self.draft.0.get(language.as_str()) != self.baseline.0.get(language.as_str())
    }

    #[must_use]
    pub fn origin(language: &LanguageId) -> Origin {
        if LspSettings::catalog().0.contains_key(language.as_str()) {
            Origin::Catalog
        } else {
            Origin::Added
        }
    }

    #[must_use]
    pub(crate) fn availability(&self, language: &LanguageId) -> Option<&Availability> {
        self.availability.get(language)
    }

    pub(crate) fn set_availability(&mut self, language: LanguageId, availability: Availability) {
        self.availability.insert(language, availability);
    }

    /// Every problem the draft has, each with whether it blocks a save.
    #[must_use]
    pub fn problems(&self) -> Vec<(Problem, Impact)> {
        problems(&self.draft, &self.baseline)
    }
}

#[cfg(test)]
mod tests;
