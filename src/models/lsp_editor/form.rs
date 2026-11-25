//! Purpose: The Language servers edit form — its fields, their tab order, the
//! suggestion list open under a field, and the values it hands back on OK.

use crate::config::{
    patch::OwnedField, Arguments, Enablement, Extensions, LanguageId, ServerSettings, ValueError,
};
use crate::lineedit::LineEditor;

mod accept;
mod field;
mod suggestions;

#[cfg(test)]
mod tests_control;
pub use accept::Accepted;
pub use field::{Direction, FormField};
pub use suggestions::{ListFocus, Suggestions};

/// Which server the form edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormTarget {
    Existing(LanguageId),
    New,
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

    /// Refuse OK: the cursor goes to `field`, and the help row says why.
    pub fn refuse(&mut self, field: FormField, error: ValueError) {
        self.focus = field;
        self.completion = None;
        self.refusal = Some(error);
    }
}
