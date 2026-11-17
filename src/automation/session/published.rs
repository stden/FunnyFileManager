//! Purpose: Share immutable application snapshots and typed job progress with the transport thread.
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Serialize)]
#[serde(transparent)]
pub(super) struct JobId(pub u64);

#[derive(Default)]
enum Job {
    #[default]
    Idle,
    Running(JobId),
    Finished {
        id: JobId,
        error: Option<String>,
    },
}

#[derive(Default)]
struct Published {
    application: Value,
    progress: Value,
    job: Job,
    revision: u64,
    sequence: u64,
}

#[derive(Clone, Default)]
pub(super) struct Shared(Arc<Mutex<Published>>);

impl Shared {
    pub(super) fn reserve(&self) -> io::Result<JobId> {
        self.with(|state| {
            if matches!(state.job, Job::Running(_)) {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "session is busy"));
            }
            if state.application.get("running").and_then(Value::as_bool) == Some(false) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "session has stopped",
                ));
            }
            state.sequence = state.sequence.saturating_add(1);
            let id = JobId(state.sequence);
            state.job = Job::Running(id);
            state.progress = Value::Null;
            state.revision = state.revision.saturating_add(1);
            Ok(id)
        })?
    }

    pub(super) fn update(&self, application: Value) -> io::Result<()> {
        self.with(|state| {
            if state.application != application {
                state.application = application;
                state.revision = state.revision.saturating_add(1);
            }
        })
    }

    pub(super) fn progress(&self, progress: Value) -> io::Result<()> {
        self.with(|state| {
            state.progress = progress;
            state.revision = state.revision.saturating_add(1);
        })
    }

    pub(super) fn is_busy(&self) -> io::Result<bool> {
        self.with(|state| matches!(state.job, Job::Running(_)))
    }

    pub(super) fn complete(&self, id: JobId, error: Option<String>) -> io::Result<()> {
        self.with(|state| {
            state.job = Job::Finished { id, error };
            state.revision = state.revision.saturating_add(1);
        })
    }

    pub(super) fn stop(&self, error: Option<String>) -> io::Result<()> {
        self.with(|state| {
            if let Some(application) = state.application.as_object_mut() {
                application.insert("running".to_owned(), json!(false));
            }
            if let Job::Running(id) = state.job {
                state.job = Job::Finished { id, error };
            }
            state.revision = state.revision.saturating_add(1);
        })
    }

    pub(super) fn snapshot(&self) -> io::Result<Value> {
        self.with(|state| {
            let mut result = state.application.as_object().cloned().unwrap_or_default();
            let (id, status, error) = match &state.job {
                Job::Idle => (None, "idle", None),
                Job::Running(id) => (Some(id), "running", None),
                Job::Finished { id, error } => (Some(id), "finished", error.as_deref()),
            };
            result.insert("revision".to_owned(), json!(state.revision));
            result.insert("progress".to_owned(), state.progress.clone());
            result.insert(
                "busy".to_owned(),
                json!(matches!(state.job, Job::Running(_))),
            );
            result.insert(
                "job".to_owned(),
                json!({"id": id, "status": status, "error": error}),
            );
            Value::Object(result)
        })
    }

    fn with<T>(&self, operation: impl FnOnce(&mut Published) -> T) -> io::Result<T> {
        self.0
            .lock()
            .map(|mut state| operation(&mut state))
            .map_err(|_error| io::Error::other("session state lock failed"))
    }
}

#[cfg(test)]
pub(super) mod tests;
