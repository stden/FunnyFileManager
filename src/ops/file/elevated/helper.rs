//! Purpose: The administrator helper's side: take each request off the pipe,
//! redo it, report its progress as it goes and its outcome at the end.

use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::rc::Rc;

use super::protocol::{self, Failure, Outcome, Reply};
use crate::models::ElevatedRetry;
use crate::progress::{Operation, Painter, Snapshot};
use crate::utils::FailedAt;

/// Serve requests from `input` until FFM closes the pipe.
///
/// # Errors
///
/// A request that cannot be read, or a reply that cannot be written.
pub(super) fn serve(mut input: impl BufRead, output: impl Write + 'static) -> io::Result<()> {
    let output = Rc::new(RefCell::new(output));
    let _painter = crate::progress::install(Box::new(Relay(Rc::clone(&output))));
    while let Some(request) = protocol::receive::<protocol::Request>(&mut input)? {
        let outcome = run_with(&request.retry, &request.choices);
        protocol::send(&mut *output.borrow_mut(), &Reply::Finished(outcome))?;
    }
    Ok(())
}

/// Sends each progress frame to FFM, which draws it.
struct Relay<W>(Rc<RefCell<W>>);

impl<W: Write> Painter for Relay<W> {
    fn paint(&mut self, snapshot: &Snapshot) {
        // A frame that cannot be sent is only a frame: the outcome after it
        // is what FFM waits for, and its failure is reported.
        drop(protocol::send(
            &mut *self.0.borrow_mut(),
            &Reply::Progress(snapshot.clone()),
        ));
    }
}

/// Redo `retry`, stopping at the first entry that fails, as FFM does.
#[cfg(test)]
pub(super) fn run(retry: &ElevatedRetry) -> Outcome {
    run_with(retry, &protocol::context_choices())
}

pub(super) fn run_with(retry: &ElevatedRetry, choices: &[super::super::dialog::Choice]) -> Outcome {
    match (&retry.settings, retry.operation) {
        (Some(settings), Operation::Copy | Operation::Move) => {
            transferred(&super::super::dialog::transfer_choosing(
                &retry.sources,
                &retry.base,
                retry.selected,
                settings,
                choices,
            ))
        }
        (None, Operation::Delete) => delete(&retry.sources),
        // Settings without a transfer, or a transfer without them: FFM
        // never sends one, and it is not read as anything it might mean.
        _ => Outcome {
            left: retry.sources.len(),
            failed: Some(Failure {
                path: retry.base.clone(),
                error: "malformed request".to_owned(),
                denial: None,
            }),
            ..Outcome::default()
        },
    }
}

fn transferred(outcome: &super::super::dialog::DialogOutcome) -> Outcome {
    let batch = &outcome.batch;
    Outcome {
        done: batch.count,
        left: batch.left,
        skipped: outcome.skipped,
        renamed: outcome.renamed,
        failed: batch.errors.first().map(|failed| Failure {
            path: failed.path().to_path_buf(),
            error: failed.error().to_owned(),
            denial: batch.denial.clone(),
        }),
    }
}

fn delete(sources: &[std::path::PathBuf]) -> Outcome {
    let _run = crate::progress::begin(Operation::Delete, sources);
    for (index, source) in sources.iter().enumerate() {
        if let Err(error) = super::super::entry::remove_path(source) {
            return Outcome {
                done: index,
                left: sources.len() - index - 1,
                failed: Some(failure(source, &error)),
                ..Outcome::default()
            };
        }
    }
    Outcome {
        done: sources.len(),
        ..Outcome::default()
    }
}

/// `error` at `source`, named by where inside it happened.
fn failure(source: &Path, error: &io::Error) -> Failure {
    let path = FailedAt::of(error).map_or(source, FailedAt::path);
    Failure {
        path: path.to_path_buf(),
        error: error.to_string(),
        denial: super::super::denied::classify(path, error),
    }
}

#[cfg(test)]
mod tests;
