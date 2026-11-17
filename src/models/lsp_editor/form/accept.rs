//! Purpose: What OK hands back: every field of the form parsed into the server
//! it describes, or the first field that does not parse.

use crate::config::{
    patch::with_owned_fields, Arguments, Extensions, LanguageId, ServerCommand, ServerSettings,
    ValueError,
};

use super::{FormField, FormTarget, ServerForm};

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

impl ServerForm {
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
}

fn new_language(text: &str, taken: impl Fn(&LanguageId) -> bool) -> Result<LanguageId, ValueError> {
    let language = LanguageId::parse(text)?;
    if taken(&language) {
        return Err(ValueError::LanguageTaken(language.as_str().to_owned()));
    }
    Ok(language)
}
