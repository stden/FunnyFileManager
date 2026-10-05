//! Purpose: The F5/F6 "Ask" choice: find which selected entries would land on
//! a name already taken, and carry the transfer out once each is answered.

use std::path::{Path, PathBuf};

use super::destination::Destination;
use super::placement::reject_transfer_onto_itself;
use crate::models::{App, Clash, ClashAnswer, Overlay};

/// Every source whose destination already holds its name.
///
/// A source sent onto itself is left out: no answer could make that work,
/// and the transfer refuses it with its own error.
pub(super) fn find(
    sources: &[PathBuf],
    base: &Path,
    settings: &crate::copy::CopyPrompt,
) -> Vec<Clash> {
    let destination = Destination::parse(settings.destination.text(), base, sources.len());
    sources
        .iter()
        .filter_map(|source| {
            let (folder, name) = destination.for_source(source);
            let dest = folder.join(name);
            let taken = crate::utils::name_is_taken(&dest)
                && reject_transfer_onto_itself(source, &dest).is_ok();
            taken.then(|| Clash::between(source, &dest))
        })
        .collect()
}

/// Answer the clash on screen; once none is left, run the transfer.
pub fn answer_existing(app: &mut App, answer: ClashAnswer) {
    let Overlay::ExistingFile(question) = &mut app.overlay else {
        return;
    };
    if !question.answer(answer) {
        return;
    }
    let question = (**question).clone();
    app.close_overlay();
    super::dialog::run(
        app,
        &question.sources,
        &question.base,
        &question.settings,
        question.answers(),
    );
}

/// Cancel the whole transfer: nothing has been copied or moved yet.
pub fn cancel_existing(app: &mut App) {
    let Overlay::ExistingFile(question) = &app.overlay else {
        return;
    };
    let purpose = question.settings.purpose;
    app.close_overlay();
    app.set_status(crate::status::Status::TransferCancelled(purpose));
}
