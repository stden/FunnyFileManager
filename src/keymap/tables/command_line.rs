//! Purpose: the command line's three tables — `command_line` while the line
//! is edited, `command_line_search` instead of it while Ctrl+R searches
//! the history, `command_line_completion` before it while the completion
//! list is open. The keys are bash's.

use crate::action::chord::NamedKey::{Backspace, Down, Enter, Esc, Tab, Up};
use crate::action::command_line::completion::CompletionCommand;
use crate::action::command_line::search::LineSearchCommand;
use crate::action::command_line::LineCommand;
use crate::keymap::chords::{ch, ctrl, named};
use crate::keymap::Origin::{Bash, Ffm};
use crate::keymap::{row, Context, Group, Table};

mod editing;

pub const COMMAND_LINE: Table<LineCommand> = Table {
    context: Context::CommandLine,
    groups: &[LINE, editing::MOVING, editing::SELECTION, editing::EDITING],
};

pub const COMMAND_LINE_SEARCH: Table<LineSearchCommand> = Table {
    context: Context::CommandLineSearch,
    groups: &[SEARCH],
};

pub const COMMAND_LINE_COMPLETION: Table<CompletionCommand> = Table {
    context: Context::CommandLineCompletion,
    groups: &[COMPLETION],
};

const LINE: Group<LineCommand> = Group {
    title: "The line and its history",
    rows: &[
        row("history_back", LineCommand::HistoryBack, &[named(Up)])
            .note(Bash("Up"), "The previous command of the history"),
        row(
            "history_forward",
            LineCommand::HistoryForward,
            &[named(Down)],
        )
        .note(Bash("Down"), "The next command, or the line being typed"),
        row(
            "history_search",
            LineCommand::HistorySearch,
            &[ctrl(ch('r'))],
        )
        .note(Bash("Ctrl+R"), "Search the history backwards"),
        row("complete", LineCommand::Complete, &[named(Tab)])
            .note(Bash("Tab"), "Complete the word under the caret"),
        row("submit", LineCommand::Submit, &[named(Enter)])
            .note(Bash("Enter"), "Run the line")
            .exit(),
        row("cancel", LineCommand::Cancel, &[named(Esc)])
            .note(Ffm, "Close the command line, dropping the line")
            .exit(),
        row("clear_line", LineCommand::CopyOrClear, &[ctrl(ch('c'))]).note(
            Bash("Ctrl+C"),
            "Copy the selection; with none, clear the line and stay",
        ),
        row(
            "delete_char_or_close",
            LineCommand::DeleteCharOrClose,
            &[ctrl(ch('d'))],
        )
        .note(
            Bash("Ctrl+D"),
            "Delete forward; close when the line is empty",
        ),
    ],
};

const SEARCH: Group<LineSearchCommand> = Group {
    title: "History search",
    rows: &[
        row(
            "search_older",
            LineSearchCommand::SearchOlder,
            &[ctrl(ch('r'))],
        )
        .note(Bash("Ctrl+R"), "The next older match"),
        row("accept", LineSearchCommand::Accept, &[named(Enter)])
            .note(Ffm, "Keep the match on the line and edit it"),
        row(
            "cancel",
            LineSearchCommand::Cancel,
            &[named(Esc), ctrl(ch('g')), ctrl(ch('c'))],
        )
        .note(Bash("Ctrl+G"), "Put back the line as it was")
        .exit(),
        row(
            "backspace",
            LineSearchCommand::Backspace,
            &[named(Backspace)],
        )
        .note(Bash("Backspace"), "Shorten what is searched for"),
    ],
};

const COMPLETION: Group<CompletionCommand> = Group {
    title: "Completion list",
    rows: &[
        row("next", CompletionCommand::Next, &[named(Down), named(Tab)])
            .note(Ffm, "Highlight the next candidate"),
        row("previous", CompletionCommand::Previous, &[named(Up)])
            .note(Ffm, "Highlight the previous candidate"),
        row("accept", CompletionCommand::Accept, &[named(Enter)])
            .note(Ffm, "Put the highlighted candidate into the line"),
        row("close", CompletionCommand::Close, &[named(Esc)])
            .note(Ffm, "Close the list, leaving the line as it is")
            .exit(),
    ],
};
