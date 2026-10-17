//! Purpose: Which text field the open dialog's cursor is on, so a paste lands there.

use crate::copy::{CopyField, CopyPrompt};
use crate::lineedit::LineEditor;
use crate::models::{ArchiveField, ArchivePrompt, ExtractField, ExtractPrompt, MenuDialog};

use super::super::{App, LspEditor, Overlay};

impl App {
    /// The text field the open dialog's cursor is on, when it is on one.
    pub fn focused_text_field(&mut self) -> Option<&mut LineEditor> {
        match &mut self.overlay {
            Overlay::CommandLine(state) => Some(&mut state.line),
            Overlay::NewFileInput(line) => Some(line),
            Overlay::NamePrompt(prompt) => prompt.focused_line_mut(),
            Overlay::MaskPrompt(prompt) => Some(prompt.line_mut()),
            Overlay::CopyPrompt(prompt) => copy_destination(prompt),
            Overlay::FindFile(dialog) => dialog.active_line_mut(),
            Overlay::ArchivePrompt(prompt) => archive_name(prompt),
            Overlay::ExtractPrompt(prompt) => extract_destination(prompt),
            Overlay::InputPrompt(prompt) => Some(prompt.input_mut()),
            Overlay::ApplyCommand(prompt) => Some(prompt.input_mut()),
            Overlay::Describe(prompt) => Some(prompt.line_mut()),
            Overlay::MenuDialog(dialog) => menu_line(dialog),
            Overlay::LspEditor(editor) => lsp_line(editor),
            Overlay::PathEditor(editor) => editor.focused_line_mut(),
            _ => None,
        }
    }
}

fn line_when(active: bool, line: &mut LineEditor) -> Option<&mut LineEditor> {
    active.then_some(line)
}

fn copy_destination(prompt: &mut CopyPrompt) -> Option<&mut LineEditor> {
    line_when(
        prompt.field == CopyField::Destination,
        &mut prompt.destination,
    )
}

fn archive_name(prompt: &mut ArchivePrompt) -> Option<&mut LineEditor> {
    line_when(prompt.field == ArchiveField::Name, &mut prompt.name)
}

fn extract_destination(prompt: &mut ExtractPrompt) -> Option<&mut LineEditor> {
    line_when(
        prompt.field == ExtractField::Destination,
        &mut prompt.destination,
    )
}

fn menu_line(dialog: &mut MenuDialog) -> Option<&mut LineEditor> {
    match dialog {
        MenuDialog::Prompt { input, .. } => Some(input),
        MenuDialog::Choices { .. } => None,
    }
}

fn lsp_line(editor: &mut LspEditor) -> Option<&mut LineEditor> {
    editor.form_mut().and_then(|form| form.focused_line_mut())
}

#[cfg(test)]
mod tests;
