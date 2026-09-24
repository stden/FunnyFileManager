//! Purpose: The Language servers edit form — its fields, their tab order, the
//! suggestion list open under a field, and the values it hands back on OK.

use crate::complete::servers::CompletedField;
use crate::config::{
    patch::{with_owned_fields, OwnedField},
    Arguments, Enablement, Extensions, LanguageId, ServerCommand, ServerSettings, ValueError,
};
use crate::lineedit::LineEditor;
use crate::models::Menu;

/// Which server the form edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormTarget {
    Existing(LanguageId),
    New,
}

/// A control of the form, in tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Language,
    Command,
    Arguments,
    Extensions,
    Enabled,
    Ok,
    Cancel,
}

impl FormField {
    pub const ORDER: [Self; 7] = [
        Self::Language,
        Self::Command,
        Self::Arguments,
        Self::Extensions,
        Self::Enabled,
        Self::Ok,
        Self::Cancel,
    ];

    /// The controls of the form's body, top to bottom, as `ui` draws them:
    /// the rule and the help row follow, and the buttons have their own row.
    pub const ROWS: [Self; 5] = [
        Self::Language,
        Self::Command,
        Self::Arguments,
        Self::Extensions,
        Self::Enabled,
    ];

    /// The control on body row `row`, which a click reaches; `None` for the
    /// rule and the help row.
    #[must_use]
    pub fn at_row(row: usize) -> Option<Self> {
        Self::ROWS.get(row).copied()
    }

    /// Whether the control takes the cursor for `target`: an existing
    /// server's language is shown but cannot be renamed.
    #[must_use]
    pub fn allows(self, target: &FormTarget) -> bool {
        !(self == Self::Language && matches!(target, FormTarget::Existing(_)))
    }

    #[must_use]
    pub const fn is_text(self) -> bool {
        matches!(
            self,
            Self::Language | Self::Command | Self::Arguments | Self::Extensions
        )
    }

    /// The suggestions this control offers, if it offers any.
    #[must_use]
    pub const fn completed(self) -> Option<CompletedField> {
        match self {
            Self::Command => Some(CompletedField::Command),
            Self::Arguments => Some(CompletedField::Arguments),
            Self::Extensions => Some(CompletedField::Extensions),
            Self::Language | Self::Enabled | Self::Ok | Self::Cancel => None,
        }
    }

    /// The help row while this control has the cursor.
    #[must_use]
    pub const fn help(self) -> &'static str {
        match self {
            Self::Language => "The LSP languageId: a-z 0-9 - _ . +",
            Self::Command => "Program name on PATH, or a full path. No shell runs it.",
            Self::Arguments => "Separate with spaces; quote one that holds a space: \"a b\"",
            Self::Extensions => "Space-separated; the dot is optional: py pyi",
            Self::Enabled => "Space toggles",
            Self::Ok => "Enter keeps these values in the list; F2 there saves them.",
            Self::Cancel => "Enter drops the changes made in this form.",
        }
    }
}

/// A move through a list or the tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Previous,
}

/// Where the keyboard is while a suggestion list is open. A passive list is
/// shown, but the field keeps Enter; an active one has a highlighted row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFocus {
    Field,
    List,
}

/// The suggestion list open under a field. `Menu` cannot be empty, so a list
/// with nothing to highlight is a list whose focus is still on the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestions {
    menu: Menu,
    focus: ListFocus,
}

impl Suggestions {
    /// A list opened by typing: nothing highlighted, Enter still presses OK.
    #[must_use]
    pub const fn passive(menu: Menu) -> Self {
        Self {
            menu,
            focus: ListFocus::Field,
        }
    }

    /// A list opened on request, its first row highlighted.
    #[must_use]
    pub const fn active(menu: Menu) -> Self {
        Self {
            menu,
            focus: ListFocus::List,
        }
    }

    #[must_use]
    pub const fn menu(&self) -> &Menu {
        &self.menu
    }

    #[must_use]
    pub const fn focus(&self) -> ListFocus {
        self.focus
    }

    /// Down / Up: a passive list becomes active on its first / last row, an
    /// active one moves its highlight, wrapping.
    pub fn step(&mut self, direction: Direction) {
        match (self.focus, direction) {
            (ListFocus::Field, Direction::Next) => self.focus = ListFocus::List,
            (ListFocus::Field, Direction::Previous) => {
                self.focus = ListFocus::List;
                self.menu.previous();
            }
            (ListFocus::List, Direction::Next) => self.menu.next(),
            (ListFocus::List, Direction::Previous) => self.menu.previous(),
        }
    }

    /// The list refiltered to `menu`, keeping its focus; an active list's
    /// highlight returns to the first row.
    #[must_use]
    pub const fn refilled(&self, menu: Menu) -> Self {
        Self {
            menu,
            focus: self.focus,
        }
    }
}

/// What a form holds once every field has parsed: the server it describes.
/// Only [`ServerForm::parse`] makes one, so its values passed the newtypes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    language: LanguageId,
    target: FormTarget,
    /// A new server with the form's values: the project root marked by
    /// `.git`, and no auto-install.
    server: ServerSettings,
}

impl Accepted {
    #[must_use]
    pub const fn language(&self) -> &LanguageId {
        &self.language
    }

    #[must_use]
    pub const fn target(&self) -> &FormTarget {
        &self.target
    }

    /// The owned fields laid over `server`; the rest are kept.
    #[must_use]
    pub fn applied_to(&self, server: ServerSettings) -> ServerSettings {
        with_owned_fields(server, &self.server)
    }

    /// A new server: the owned fields, the project root marked by `.git`, and
    /// no auto-install.
    #[must_use]
    pub fn new_server(&self) -> ServerSettings {
        self.server.clone()
    }
}

/// The edit form of one server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerForm {
    target: FormTarget,
    language: LineEditor,
    command: LineEditor,
    arguments: LineEditor,
    extensions: LineEditor,
    enabled: Enablement,
    focus: FormField,
    completion: Option<Suggestions>,
    refusal: Option<ValueError>,
}

impl ServerForm {
    /// The form for an existing server, or the field holding a control
    /// character, which a one-line field could not show.
    pub fn for_server(language: &LanguageId, server: &ServerSettings) -> Result<Self, OwnedField> {
        let control = |text: &str| text.chars().any(char::is_control);
        if control(&server.command) {
            return Err(OwnedField::Command);
        }
        if server.args.iter().any(|argument| control(argument)) {
            return Err(OwnedField::Args);
        }
        if server.extensions.iter().any(|extension| control(extension)) {
            return Err(OwnedField::Extensions);
        }
        Ok(Self {
            target: FormTarget::Existing(language.clone()),
            language: LineEditor::new(language.as_str()),
            command: LineEditor::new(&server.command),
            arguments: LineEditor::new(&Arguments::to_line(&server.args)),
            extensions: LineEditor::new(&Extensions::to_line(&server.extensions)),
            enabled: server.enabled,
            focus: FormField::Command,
            completion: None,
            refusal: None,
        })
    }

    /// An empty form for a server to add, the cursor on its language.
    #[must_use]
    pub fn new_server() -> Self {
        Self {
            target: FormTarget::New,
            language: LineEditor::default(),
            command: LineEditor::default(),
            arguments: LineEditor::default(),
            extensions: LineEditor::default(),
            enabled: Enablement::Enabled,
            focus: FormField::Language,
            completion: None,
            refusal: None,
        }
    }

    #[must_use]
    pub const fn target(&self) -> &FormTarget {
        &self.target
    }

    #[must_use]
    pub const fn focus(&self) -> FormField {
        self.focus
    }

    #[must_use]
    pub const fn enabled(&self) -> Enablement {
        self.enabled
    }

    #[must_use]
    pub const fn completion(&self) -> Option<&Suggestions> {
        self.completion.as_ref()
    }

    pub const fn completion_mut(&mut self) -> Option<&mut Suggestions> {
        self.completion.as_mut()
    }

    pub fn set_completion(&mut self, completion: Option<Suggestions>) {
        self.completion = completion;
    }

    /// The text of a text control; `None` for the checkbox and the buttons.
    #[must_use]
    pub const fn line(&self, field: FormField) -> Option<&LineEditor> {
        match field {
            FormField::Language => Some(&self.language),
            FormField::Command => Some(&self.command),
            FormField::Arguments => Some(&self.arguments),
            FormField::Extensions => Some(&self.extensions),
            FormField::Enabled | FormField::Ok | FormField::Cancel => None,
        }
    }

    /// The text control with the cursor, for editing.
    pub const fn focused_line_mut(&mut self) -> Option<&mut LineEditor> {
        match self.focus {
            FormField::Language => Some(&mut self.language),
            FormField::Command => Some(&mut self.command),
            FormField::Arguments => Some(&mut self.arguments),
            FormField::Extensions => Some(&mut self.extensions),
            FormField::Enabled | FormField::Ok | FormField::Cancel => None,
        }
    }

    /// The next or previous control this form lets the cursor on, wrapping.
    /// Moving closes the suggestion list and forgets the last refusal.
    pub fn step_focus(&mut self, direction: Direction) {
        let order: Vec<FormField> = FormField::ORDER
            .into_iter()
            .filter(|field| field.allows(&self.target))
            .collect();
        self.focus = crate::cycle::step(&order, self.focus, direction == Direction::Next);
        self.completion = None;
        self.refusal = None;
    }

    /// Put the cursor on `field`, as a click does. A move closes the
    /// suggestion list and forgets the last refusal, as Tab does; staying
    /// where it is, or a control this form skips, changes nothing.
    pub fn focus_on(&mut self, field: FormField) {
        if field != self.focus && field.allows(&self.target) {
            self.focus = field;
            self.completion = None;
            self.refusal = None;
        }
    }

    pub const fn toggle_enabled(&mut self) {
        self.enabled = self.enabled.toggled();
    }

    /// The help row: why the last OK was refused, else the control's help.
    #[must_use]
    pub fn help(&self) -> String {
        self.refusal
            .as_ref()
            .map_or_else(|| self.focus.help().to_owned(), ToString::to_string)
    }

    /// The reason the last OK was refused, while it stands.
    #[must_use]
    pub const fn refusal(&self) -> Option<&ValueError> {
        self.refusal.as_ref()
    }

    /// Every field parsed, or the first that does not, with why. `taken`
    /// says whether a new server's language is already in the table.
    pub fn parse(
        &self,
        taken: impl Fn(&LanguageId) -> bool,
    ) -> Result<Accepted, (FormField, ValueError)> {
        let language = match &self.target {
            FormTarget::Existing(language) => language.clone(),
            FormTarget::New => new_language(self.language.text(), taken)
                .map_err(|error| (FormField::Language, error))?,
        };
        let command = ServerCommand::parse(self.command.text())
            .map_err(|error| (FormField::Command, error))?;
        let args = Arguments::parse_line(self.arguments.text())
            .map_err(|error| (FormField::Arguments, error))?;
        let extensions = Extensions::parse_line(self.extensions.text())
            .map_err(|error| (FormField::Extensions, error))?;
        let server = ServerSettings {
            enabled: self.enabled,
            command: command.into(),
            args: args.into(),
            extensions: extensions.into(),
            root_markers: vec![".git".to_owned()],
            install: None,
        };
        Ok(Accepted {
            language,
            target: self.target.clone(),
            server,
        })
    }

    /// Refuse OK: the cursor goes to `field`, and the help row says why.
    pub fn refuse(&mut self, field: FormField, error: ValueError) {
        self.focus = field;
        self.completion = None;
        self.refusal = Some(error);
    }
}

fn new_language(text: &str, taken: impl Fn(&LanguageId) -> bool) -> Result<LanguageId, ValueError> {
    let language = LanguageId::parse(text)?;
    if taken(&language) {
        return Err(ValueError::LanguageTaken(language.as_str().to_owned()));
    }
    Ok(language)
}
