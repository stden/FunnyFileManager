//! Purpose: Entering and leaving the dialog's form: opening it on a server,
//! and OK writing its values into the draft.

use crate::config::LanguageId;

use super::{FormTarget, LspEditor, Mode, Notice, Problem, ServerForm};

impl LspEditor {
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
}
