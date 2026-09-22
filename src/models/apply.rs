//! Purpose: The apply-command prompt (`Ctrl+G`) and Far's `!` substitutions it expands per entry.
//!
//! State and pure text work only: the runs live in `ops::apply`, and the keys
//! that move through the dialog in `popup::apply`.
//!
//! The token grammar follows Far's `fnparse.cpp`. Filesystem aliases and
//! temporary lists are supplied by operations, never guessed by the parser.

use std::path::{Path, PathBuf};

use crate::lineedit::LineEditor;

mod context;
mod symbols;

pub(crate) use context::{Context, Names, PanelContext, Paths};
pub(crate) use symbols::{ListEncoding, ListOptions, NameForm, Symbol, Token, Tokens};

/// The command as typed, and which button the cursor is on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyPrompt {
    /// The same line editor as the `:` command line and the generic prompts,
    /// so the field edits mid-line rather than only at its end.
    input: LineEditor,
    /// The cursor is on Cancel rather than on OK.
    on_cancel: bool,
}

impl ApplyPrompt {
    #[must_use]
    pub fn command(&self) -> &str {
        self.input.text()
    }

    #[must_use]
    pub const fn input(&self) -> &LineEditor {
        &self.input
    }

    pub const fn input_mut(&mut self) -> &mut LineEditor {
        &mut self.input
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        self.on_cancel
    }

    pub const fn toggle_button(&mut self) {
        self.on_cancel = !self.on_cancel;
    }

    /// Put the cursor on Cancel (or back on OK): what a click on a button
    /// does before Enter, so an old keyboard focus cannot make it act as the
    /// other button.
    pub const fn focus_cancel(&mut self, on_cancel: bool) {
        self.on_cancel = on_cancel;
    }
}

/// Convenience input for expanding a template against a single panel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// Every name the command applies to, in panel order.
    pub names: Vec<String>,
    /// The file `!@!` names, written only when the template asks for it —
    /// `None` expands the symbol to nothing rather than to a path that was
    /// never created.
    pub list_file: Option<PathBuf>,
}

/// Whether `template` uses the list file, so the caller can write it once
/// before the runs and remove it after the last.
#[must_use]
pub fn wants_list_file(template: &str) -> bool {
    Tokens::new(template).any(|token| matches!(token, Token::List(_)))
}

/// Expand every symbol in `template` for `entry_path`, one of `selected`.
///
/// Nothing is quoted on the caller's behalf (Far's note 1): `"!.!"` is the
/// user's to write. `!&` is the one exception, because Far itself quotes it.
#[must_use]
pub fn substitute(template: &str, entry_path: &Path, selected: &Selection) -> String {
    let context = Context::single(entry_path, &selected.names);
    let result = expand(template, &context, |_, _| {
        Ok::<_, std::convert::Infallible>(
            selected
                .list_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        )
    });
    match result {
        Ok(command) => command,
        Err(never) => match never {},
    }
}

/// Expand without reparsing inserted names. List creation is an injected
/// effect, allowing the operations layer to own temporary-file lifetimes.
pub(crate) fn expand<E>(
    template: &str,
    context: &Context,
    mut list: impl FnMut(&PanelContext, ListOptions) -> Result<String, E>,
) -> Result<String, E> {
    let mut expanded = String::with_capacity(template.len());
    let mut side = context.active;
    for token in Tokens::new(template) {
        match token {
            Token::Text(text) => expanded.push_str(text),
            Token::Panel(panel) => side = context.side(panel),
            Token::Symbol(symbol) => expanded.push_str(&context.panel(side).expand(symbol)),
            Token::List(options) => expanded.push_str(&list(context.panel(side), options)?),
        }
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests;
