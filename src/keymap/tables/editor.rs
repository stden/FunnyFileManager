//! Purpose: the `editor` table — the F4 editor: the caret with and without
//! a selection, the clipboard, the file, the text keys and the language
//! server's.

use crate::action::chord::NamedKey::{Backspace, Del, Enter, Esc, Ins, Space, Tab};
use crate::action::editor::EditorCommand;
use crate::keymap::chords::{alt, ch, ctrl, f, named, shift};
use crate::keymap::Origin::{Far, FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};
use crate::models::LspFeature;

mod caret;

pub const EDITOR: Table<EditorCommand> = Table {
    context: Context::Editor,
    groups: &[caret::MOVING, caret::SELECTING, FILE, TEXT, LANGUAGE_SERVER],
};

const FILE: Group<EditorCommand> = Group {
    title: "Clipboard, file and search",
    rows: &[
        row("select_all", EditorCommand::SelectAll, &[ctrl(ch('a'))])
            .note(FarSame, "Select the whole text"),
        row(
            "copy",
            EditorCommand::Copy,
            &[ctrl(ch('c')), ctrl(named(Ins))],
        )
        .note(Far("Ctrl+Ins"), "Copy the selection"),
        row(
            "paste",
            EditorCommand::Paste,
            &[ctrl(ch('v')), shift(named(Ins))],
        )
        .note(Far("Shift+Ins"), "Paste the clipboard"),
        row(
            "insert_file_name",
            EditorCommand::InsertFileName,
            &[ctrl(ch('f'))],
        )
        .note(
            Far("Ctrl+F"),
            "Put the file's full name at the caret and on the clipboard",
        ),
        row(
            "format_document",
            EditorCommand::FormatDocument,
            &[ctrl(alt(ch('l')))],
        )
        .note(Ffm, "Format the document, as JSON or XML"),
        row("save", EditorCommand::Save, &[f(2)]).note(FarSame, "Save the file"),
        row("close", EditorCommand::Close, &[named(Esc), f(10), f(4)])
            .note(FarSame, "Close the editor, asking when the text changed")
            .exit(),
        row("search", EditorCommand::Search, &[f(7)]).note(FarSame, "Search the text"),
        row("search_again", EditorCommand::RepeatSearch, &[shift(f(7))])
            .note(FarSame, "Search for the same text again"),
        row("toggle_code_page", EditorCommand::ToggleCodePage, &[f(8)])
            .note(FarSame, "Toggle the OEM and ANSI code pages"),
        row(
            "choose_encoding",
            EditorCommand::ChooseEncoding,
            &[shift(f(8))],
        )
        .note(FarSame, "Choose the code page in a dialog"),
    ],
};

const TEXT: Group<EditorCommand> = Group {
    title: "Text",
    rows: &[
        row("newline", EditorCommand::Newline, &[named(Enter)]).note(FarSame, "Start a line"),
        row("backspace", EditorCommand::Backspace, &[named(Backspace)])
            .note(FarSame, "Delete the character before the caret"),
        row("delete_char", EditorCommand::Delete, &[named(Del)])
            .note(FarSame, "Delete the character under the caret"),
        row("indent", EditorCommand::Indent, &[named(Tab)]).note(FarSame, "Indent"),
    ],
};

const LANGUAGE_SERVER: Group<EditorCommand> = Group {
    title: "Language server",
    rows: &[
        row(
            "complete",
            EditorCommand::Lsp(LspFeature::Completion(
                crate::models::CompletionTrigger::Manual,
            )),
            &[ctrl(named(Space))],
        )
        .note(Ffm, "Language server: complete"),
        row(
            "hover",
            EditorCommand::Lsp(LspFeature::Hover),
            &[ctrl(ch('k'))],
        )
        .note(Ffm, "Language server: what is under the caret"),
        row(
            "go_to_definition",
            EditorCommand::Lsp(LspFeature::Definition),
            &[ctrl(ch(']')), ctrl(ch('5'))],
        )
        .note(Ffm, "Language server: go to definition"),
        row(
            "restart_language_server",
            EditorCommand::RestartLsp,
            &[ctrl(alt(ch('r')))],
        )
        .note(Ffm, "Language server: start it again"),
    ],
};
