//! Purpose: path messages of outcomes named by what an operation worked on — the archive, the program, or the first entry a batch failed on.

use super::PathMessage;
use crate::path_display::label;
use crate::status::display::{archive_noun, ERROR_PREFIX};
use crate::status::{FailedEntry, Status};

/// A partly failed batch: its `summary`, then the first failure — the path it
/// failed on, quoted so a narrow row shortens that and keeps the reason.
fn first_failure<'a>(summary: &str, first_error: &'a FailedEntry) -> PathMessage<'a> {
    PathMessage::quoting(
        format!("{summary}: "),
        label(first_error.path()),
        format!(": {}", first_error.error()),
    )
}

/// The one shape a partly failed copy, move, delete or wipe reports in: the
/// verb, the successes, the failures, and the first failure spelled out.
fn with_errors<'a>(
    verb: &str,
    count: usize,
    error_count: usize,
    first_error: &'a FailedEntry,
) -> PathMessage<'a> {
    first_failure(
        &format!("{verb} {count} item(s), {error_count} error(s)"),
        first_error,
    )
}

impl Status {
    /// Archive outcomes, named by the archive or the folder they unpacked to.
    pub(super) fn archive_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::Extracted { count, destination } => PathMessage::quoting(
                format!("Extracted {count} {} to ", archive_noun(*count)),
                label(destination),
                "",
            ),
            Self::AddedToArchive {
                count,
                archive,
                kept,
            } => {
                let leftovers = if *kept > 0 {
                    format!("; {kept} could not be deleted")
                } else {
                    String::new()
                };
                PathMessage::quoting(
                    format!("Added {count} item(s) to "),
                    label(archive),
                    leftovers,
                )
            }
            _ => return None,
        })
    }

    /// How a program started with Enter ended, named by the program.
    pub(super) fn run_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::ProgramExited { program, status } => {
                PathMessage::quoting("", label(program), format!(": {status}"))
            }
            Self::CannotRun { program, error } => PathMessage::quoting(
                format!("{ERROR_PREFIX}Cannot run "),
                label(program),
                format!(": {error}"),
            ),
            _ => return None,
        })
    }

    /// Copies, moves, deletes and wipes that partly failed, named by the
    /// first entry that failed.
    pub(super) fn failure_path_message(&self) -> Option<PathMessage<'_>> {
        Some(match self {
            Self::CopiedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Copied", *count, *error_count, first_error),
            Self::MovedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Moved", *count, *error_count, first_error),
            Self::DeletedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Deleted", *count, *error_count, first_error),
            Self::WipedWithErrors {
                count,
                error_count,
                first_error,
            } => with_errors("Wiped", *count, *error_count, first_error),
            _ => return None,
        })
    }

    /// Ctrl+A that failed on some entries, named by the first.
    pub(super) fn attributes_path_message(&self) -> Option<PathMessage<'_>> {
        let Self::AttributesSetWithErrors {
            count,
            error_count,
            first_error,
        } = self
        else {
            return None;
        };
        Some(first_failure(
            &format!("Attributes set on {count} object(s), {error_count} failed"),
            first_error,
        ))
    }
}
