//! Purpose: The F5/F6 dialog's transfer: send each selected entry where the
//! dialog says, and report what was placed, renamed or skipped.

use std::{fs, io, path::Path};

use super::destination::Destination;
use super::entry::reject_transfer_into_itself;
use super::placement::{place, place_over_existing, Placed};
use super::transfer::BatchOutcome;
use super::{get_source_paths, refresh_after_operation};
use crate::copy::Purpose;
use crate::models::App;
use crate::status::{Landed, Status};

/// Carry out what the F5/F6 dialog was filled in with.
///
/// The destination is whatever was typed, which may be neither panel — that is
/// the reason the dialog exists — and may be a folder or a new name; see
/// [`Destination`]. A relative path is taken against the active panel,
/// because that is where the user is looking when they type one.
pub fn copy_with(app: &mut App, settings: &crate::copy::CopyPrompt) {
    let sources = get_source_paths(app);
    if sources.is_empty() {
        return;
    }

    let base = &app.get_active_panel_ref().path;
    let destination = Destination::parse(&settings.destination, base, sources.len());
    let mut outcome = DialogOutcome::default();
    for source in &sources {
        outcome.record(source, copy_one(source, &destination, settings));
    }
    report_copy_outcome(app, &outcome, settings.purpose);
    refresh_after_operation(app);
}

/// What a dialog transfer achieved, item by item.
#[derive(Default)]
struct DialogOutcome {
    batch: BatchOutcome,
    /// Left alone by the policy, at any depth of a merged folder. Not an
    /// error and not a transfer either, so it is counted apart, or the status
    /// line would claim work not done.
    skipped: usize,
    /// Put beside an existing file under a new name — which the status line
    /// has to say, because the name the user looks for is not there.
    renamed: usize,
}

impl DialogOutcome {
    fn record(&mut self, source: &Path, result: io::Result<Placed>) {
        if let Ok(placed) = &result {
            self.skipped = self.skipped.saturating_add(placed.skipped());
        }
        let result = match result {
            Ok(Placed::Skipped) => return,
            Ok(Placed::Beside(dest)) => {
                self.renamed += 1;
                Ok(dest)
            }
            Ok(Placed::AsSent(dest) | Placed::Merged { into: dest, .. }) => Ok(dest),
            Err(error) => Err(error),
        };
        self.batch.record(source, result);
    }
}

fn report_copy_outcome(app: &mut App, outcome: &DialogOutcome, purpose: Purpose) {
    let batch = &outcome.batch;
    app.get_active_panel().clear_marks();
    if let Some(first_error) = batch.errors.first().cloned() {
        let (count, error_count) = (batch.count, batch.errors.len());
        app.set_status(match purpose {
            Purpose::Copy => Status::CopiedWithErrors {
                count,
                error_count,
                first_error,
            },
            Purpose::Move => Status::MovedWithErrors {
                count,
                error_count,
                first_error,
            },
        });
        return;
    }
    app.set_status(Status::Transferred {
        purpose,
        landed: Landed::of(batch.count, batch.first_dest.clone()),
        skipped: outcome.skipped,
        renamed: outcome.renamed,
    });
}

/// Transfer one entry to where the dialog sends it.
fn copy_one(
    source: &Path,
    destination: &Destination,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    let (folder, name) = destination.for_source(source);
    reject_transfer_into_itself(source, folder)?;
    fs::create_dir_all(folder).map_err(|error| crate::utils::FailedAt::wrap(folder, error))?;

    let dest = folder.join(name);
    // Not `exists`, which follows a link: a link that points nowhere still
    // holds the name, and a rename would replace it without a word.
    if crate::utils::name_is_taken(&dest) {
        return place_over_existing(source, &dest, settings);
    }
    place(source, &dest, settings)?;
    Ok(Placed::AsSent(dest))
}
