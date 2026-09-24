//! Purpose: The Language servers dialog's state — the servers loaded from
//! `settings.yaml`, the draft being edited, the selection, and the mode.
//!
//! Private fields with accessors for `ui` and mutations for `ops` and
//! `popup`. Nothing here reads a file or the machine: availability arrives
//! from `ops`, and the draft reaches the file only through a save there.

use std::{collections::BTreeMap, path::PathBuf};

use crate::config::{patch::with_owned_fields, LanguageId, LspSettings, ServerSettings};
use crate::lsp::Availability;
use crate::models::ExitChoice;
use crate::scroll::ScrollTop;

mod form;
mod problems;
pub use form::{Accepted, Direction, FormField, FormTarget, ListFocus, ServerForm, Suggestions};
pub use problems::{problems, Impact, Problem};

/// Whether the dialog may write the settings file, and which one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Writable { path: PathBuf },
    ReadOnly(ReadOnlyReason),
}

/// Why the dialog only shows the servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOnlyReason {
    /// A session without a home keeps its settings in memory.
    NoSettingsFile,
    /// The file at `path` cannot be read, or does not parse; `reason` says
    /// why, naming the file.
    Document { path: PathBuf, reason: String },
}

impl std::fmt::Display for ReadOnlyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSettingsFile => f.write_str("no settings file for this session"),
            Self::Document { reason, .. } => f.write_str(reason),
        }
    }
}

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

/// A move of the list selection or the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
}

/// What the last frame drew, which paging needs. Recorded by the renderer,
/// like the dialog mouse; a frame draws the list screen or the F3 view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drawn {
    /// The list screen, its server list `page` rows tall.
    List { page: usize },
    /// The F3 view: `lines` in all, `page` of them on screen.
    View { lines: usize, page: usize },
}

impl Default for Drawn {
    fn default() -> Self {
        Self::List { page: 1 }
    }
}

impl Drawn {
    /// How far a list page moves the selection: at least one row.
    fn list_page(self) -> usize {
        match self {
            Self::List { page } => page.max(1),
            Self::View { .. } => 1,
        }
    }

    /// How far a view page scrolls: at least one line.
    fn view_page(self) -> usize {
        match self {
            Self::View { page, .. } => page.max(1),
            Self::List { .. } => 1,
        }
    }

    /// The last line the view can start at, with its last line at the bottom.
    fn last_view_top(self) -> usize {
        match self {
            Self::View { lines, page } => lines.saturating_sub(page),
            Self::List { .. } => 0,
        }
    }
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

    /// Open the form on `target`, or say why it cannot open.
    pub fn open_form(&mut self, target: &FormTarget) -> Result<(), Notice> {
        let form = match target {
            FormTarget::New => ServerForm::new_server(),
            FormTarget::Existing(language) => {
                let server = self
                    .draft
                    .0
                    .get(language.as_str())
                    .ok_or_else(|| Notice::Error(format!("{language} is not in the list")))?;
                ServerForm::for_server(language, server).map_err(|field| {
                    Notice::Error(
                        Problem::ControlCharacter {
                            language: language.clone(),
                            field,
                        }
                        .to_string(),
                    )
                })?
            }
        };
        self.mode = Mode::Form(Box::new(form));
        Ok(())
    }

    /// OK: the form's values into the draft, and back to the list on that
    /// server. A value that does not parse keeps the form open on its field
    /// and changes nothing.
    pub fn accept_form(&mut self) -> Option<LanguageId> {
        let draft = &self.draft;
        let Mode::Form(form) = &mut self.mode else {
            return None;
        };
        let accepted = match form.parse(|language| draft.0.contains_key(language.as_str())) {
            Ok(accepted) => accepted,
            Err((field, error)) => {
                form.refuse(field, error);
                return None;
            }
        };
        let language = accepted.language().clone();
        let server = match (accepted.target(), self.draft.0.get(language.as_str())) {
            (FormTarget::Existing(_), Some(server)) => accepted.applied_to(server.clone()),
            _ => accepted.new_server(),
        };
        self.draft.0.insert(language.as_str().to_owned(), server);
        self.selected = Some(language.clone());
        self.mode = Mode::Browse;
        Some(language)
    }

    /// Record what the frame just drew, and keep the view in range.
    pub fn record_drawn(&mut self, drawn: Drawn) {
        self.drawn = drawn;
        let last = drawn.last_view_top();
        if let Mode::View(top) = &mut self.mode {
            *top = ScrollTop::TOP.down(top.lines(), last);
        }
    }

    /// Scroll the F3 view within what the last frame drew.
    pub fn scroll_view(&mut self, step: Step) {
        let last = self.drawn.last_view_top();
        let page = self.drawn.view_page();
        let Mode::View(top) = &mut self.mode else {
            return;
        };
        *top = match step {
            Step::Up => top.up(1),
            Step::Down => top.down(1, last),
            Step::PageUp => top.up(page),
            Step::PageDown => top.down(page, last),
            Step::First => ScrollTop::TOP,
            Step::Last => ScrollTop::TOP.down(last, last),
        };
    }
}

#[cfg(test)]
mod tests;
