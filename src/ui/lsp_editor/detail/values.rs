//! Purpose: The labelled rows under a server's detail heading — command,
//! arguments, status and what FFM would run, the auto-install recipe and its
//! problems — each value as coloured pieces.

use std::path::Path;

use super::super::super::{code::token_style, palette};
use super::{
    impact_style, plain, tone_style, Piece, ARGUMENTS, AUTO_INSTALL, COMMAND, EXECUTABLE,
    LAUNCH_ARGUMENTS, PROBLEM, STATUS,
};
use crate::config::{Arguments, LanguageId, ServerSettings};
use crate::highlight::{Highlighter, Token};
use crate::lsp::Availability;
use crate::models::LspEditor;

/// What an empty value reads as.
const NOTHING: &str = "(none)";
/// What a server without an auto-install recipe reads as.
const OFF: &str = "off";

/// The labelled rows under the heading.
pub(super) fn rows(
    editor: &LspEditor,
    language: &LanguageId,
    server: &ServerSettings,
) -> Vec<(&'static str, Vec<Piece>)> {
    let mut rows = vec![
        (COMMAND, command_pieces(&server.command)),
        (ARGUMENTS, argument_pieces(&server.args)),
    ];
    if let Some(availability) = editor.availability(language) {
        rows.extend(availability_rows(availability));
    }
    rows.push((AUTO_INSTALL, recipe_pieces(server)));
    for (problem, impact) in editor.problems() {
        if problem.involves(language) {
            rows.push((PROBLEM, vec![(problem.to_string(), impact_style(impact))]));
        }
    }
    rows
}

fn command_pieces(command: &str) -> Vec<Piece> {
    if command.trim().is_empty() {
        return vec![(NOTHING.to_owned(), plain(palette::DISABLED_FG))];
    }
    vec![(command.to_owned(), plain(palette::DIRECTORY))]
}

/// Arguments in Far's switch green, written as the form reads them back.
fn argument_pieces(arguments: &[String]) -> Vec<Piece> {
    if arguments.is_empty() {
        return vec![(NOTHING.to_owned(), plain(palette::DISABLED_FG))];
    }
    vec![(Arguments::to_line(arguments), token_style(Token::Switch))]
}

/// The Status row, and what FFM would run when it would run something.
fn availability_rows(availability: &Availability) -> Vec<(&'static str, Vec<Piece>)> {
    let mut rows = vec![(
        STATUS,
        vec![(
            availability.label().to_owned(),
            tone_style(availability.tone()),
        )],
    )];
    if let Some(resolved) = availability.resolved() {
        let executable = resolved.executable.display().to_string();
        rows.push((EXECUTABLE, vec![(executable, token_style(Token::Str))]));
        rows.push((LAUNCH_ARGUMENTS, argument_pieces(&resolved.args)));
    }
    rows
}

/// The auto-install recipe as compact JSON, highlighted as JSON, or `off`.
fn recipe_pieces(server: &ServerSettings) -> Vec<Piece> {
    let Some(json) = server
        .install
        .as_ref()
        .and_then(|recipe| serde_json::to_string(recipe).ok())
    else {
        return vec![(OFF.to_owned(), plain(palette::DISABLED_FG))];
    };
    Highlighter::new(Path::new("recipe.json"))
        .line(&json)
        .into_iter()
        .map(|(text, token)| (text.to_owned(), token_style(token)))
        .collect()
}
