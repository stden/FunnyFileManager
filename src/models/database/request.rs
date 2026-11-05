//! Purpose: The explorer's requests to a database, and their answers.
//!
//! The one-slot queue that holds the request in flight is state only; the job that
//! runs a request is in `application::database_jobs`.

use super::meta::{Column, Page, Schema, TableRef};
use super::ConnectionSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestId(u64);

#[derive(Debug, Clone)]
pub(crate) enum Task {
    Schemas(ConnectionSpec),
    Columns {
        spec: ConnectionSpec,
        table: TableRef,
    },
    Rows {
        spec: ConnectionSpec,
        table: TableRef,
        offset: usize,
    },
}

#[derive(Debug)]
pub(crate) enum Reply {
    Schemas(Vec<Schema>),
    Columns(Vec<Column>),
    Rows(Page),
}

#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) id: RequestId,
    pub(crate) task: Task,
}

#[derive(Debug, Clone, Default)]
enum Work {
    #[default]
    Idle,
    Queued(Box<Request>),
    Running(RequestId),
}

/// One request at a time: queued, then running until its answer is taken.
#[derive(Debug, Clone, Default)]
pub(crate) struct Requests {
    next: u64,
    work: Work,
}

impl Requests {
    pub(crate) const fn is_idle(&self) -> bool {
        matches!(self.work, Work::Idle)
    }

    /// Queue `task`; `None` while another request is queued or running.
    pub(crate) fn queue(&mut self, task: Task) -> Option<RequestId> {
        if !self.is_idle() {
            return None;
        }
        let next = self.next.checked_add(1)?;
        self.next = next;
        let id = RequestId(next);
        self.work = Work::Queued(Box::new(Request { id, task }));
        Some(id)
    }

    /// Hand the queued request to a worker, which makes it the running one.
    pub(crate) fn take(&mut self) -> Option<Request> {
        if !matches!(self.work, Work::Queued(_)) {
            return None;
        }
        match std::mem::replace(&mut self.work, Work::Idle) {
            Work::Queued(request) => {
                self.work = Work::Running(request.id);
                Some(*request)
            }
            other => {
                self.work = other;
                None
            }
        }
    }

    /// Mark `id` done; `false` when it is not the running request.
    pub(crate) fn finish(&mut self, id: RequestId) -> bool {
        if matches!(self.work, Work::Running(running) if running == id) {
            self.work = Work::Idle;
            return true;
        }
        false
    }
}
