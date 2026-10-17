//! Purpose: The F5/F6 dialog's transfer: send each selected entry where the
//! dialog says, and report what was placed, renamed or skipped.

use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};

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

    let base = app.get_active_panel_ref().path.clone();
    if settings.existing == crate::copy::ExistingFiles::Ask {
        let clashes = super::clashes::find(&sources, &base, settings);
        if !clashes.is_empty() {
            let question =
                crate::models::ExistingFile::new(settings.clone(), sources, base, clashes);
            app.set_overlay(crate::models::Overlay::ExistingFile(Box::new(question)));
            return;
        }
    }
    run(app, &sources, &base, settings, &[]);
}

/// Carry the transfer out, with `choices` saying what to do about each
/// source whose destination was already taken.
pub(super) fn run(
    app: &mut App,
    sources: &[PathBuf],
    base: &Path,
    settings: &crate::copy::CopyPrompt,
    choices: &[Choice],
) {
    let outcome = transfer_choosing(sources, base, sources.len(), settings, choices);
    let batch = &outcome.batch;
    let remaining = super::remaining(sources, batch.left);
    // An administrator's retry goes on as the user answered for the entry
    // that failed.
    let retry_settings = remaining.first().map_or_else(
        || Cow::Borrowed(settings),
        |first| for_source(first, settings, choices),
    );
    let stopped = batch.errors.first().map(|failed| super::Stop {
        operation: crate::progress::Operation::of(settings.purpose),
        failed,
        denial: batch.denial.clone(),
        done: batch.count,
        remaining,
        base,
        selected: sources.len(),
        settings: Some(&retry_settings),
    });
    report_copy_outcome(app, &outcome, settings.purpose);
    if let Some(stopped) = &stopped {
        super::stop(app, stopped);
    }
    refresh_after_operation(app);
}

/// What the user answered for one source whose destination was taken.
pub(super) type Choice = (PathBuf, crate::copy::ExistingFiles);

/// The dialog's settings for `source`, with its own answer in place of the
/// "already existing files" choice when it has one.
fn for_source<'a>(
    source: &Path,
    settings: &'a crate::copy::CopyPrompt,
    choices: &[Choice],
) -> Cow<'a, crate::copy::CopyPrompt> {
    match choices.iter().find(|(path, _)| path == source) {
        Some((_, existing)) if *existing != settings.existing => {
            let mut own = settings.clone();
            own.existing = *existing;
            Cow::Owned(own)
        }
        _ => Cow::Borrowed(settings),
    }
}

/// Send `sources` where `settings` say, stopping at the first that fails,
/// with the user's answer for each source in `choices`. `selected` is how
/// many entries the operation was given, which decides whether a typed
/// destination is a folder or a new name ([`Destination`]).
pub(super) fn transfer_choosing(
    sources: &[PathBuf],
    base: &Path,
    selected: usize,
    settings: &crate::copy::CopyPrompt,
    choices: &[Choice],
) -> DialogOutcome {
    let destination = Destination::parse(settings.destination.text(), base, selected);
    let mut outcome = DialogOutcome::default();
    let _run = crate::progress::begin(crate::progress::Operation::of(settings.purpose), sources);
    for (index, source) in sources.iter().enumerate() {
        let own = for_source(source, settings, choices);
        outcome.record(source, copy_one(source, &destination, &own));
        crate::progress::source_done();
        if outcome.batch.stopped() {
            outcome.batch.left = sources.len() - index - 1;
            break;
        }
    }
    outcome
}

/// What a dialog transfer achieved, item by item.
#[derive(Default)]
pub(super) struct DialogOutcome {
    pub(super) batch: BatchOutcome,
    /// Left alone by the policy, at any depth of a merged folder. Not an
    /// error and not a transfer either, so it is counted apart, or the status
    /// line would claim work not done.
    pub(super) skipped: usize,
    /// Put beside an existing file under a new name — which the status line
    /// has to say, because the name the user looks for is not there.
    pub(super) renamed: usize,
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
    if let Some(first_error) = batch.errors.first() {
        let status =
            super::transfer::stopped_status(purpose, batch.count, batch.errors.len(), first_error);
        app.set_status(status);
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
