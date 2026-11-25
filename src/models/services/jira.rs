//! Purpose: Jira issue browser, filter, multi-selection, and JQL prompt modal state.

mod browse;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::lineedit::LineEditor;
use crate::values::jira::{
    ExportReport, JiraBusy, JiraError, JiraIssue, JiraReply, JiraRequest, JiraUser,
};

/// Numbers Jira requests inside one dialog session so stale replies are dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JiraRequestId(u64);

impl JiraRequestId {
    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Active sub-view inside the Jira browser dialog.
#[derive(Clone)]
pub enum JiraMode {
    Browse,
    Detail {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
    EditJql(LineEditor),
}

/// Interactive Jira ticket list, detail viewer, and exporter dialog.
#[derive(Clone)]
pub struct JiraDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) latest: JiraRequestId,
    pub(crate) setup_error: Option<String>,
    pub(crate) issues: Vec<JiraIssue>,
    pub(crate) user: Option<JiraUser>,
    pub(crate) base_url: String,
    pub(crate) jql: String,
    pub(crate) page_size: u16,
    pub(crate) export_dir: PathBuf,
    pub(crate) filter: String,
    pub(crate) cursor: usize,
    pub(crate) marks: BTreeSet<String>,
    pub(crate) mode: JiraMode,
    pub(crate) busy: Option<JiraBusy>,
    pub(crate) queued: Option<(JiraRequestId, JiraRequest)>,
    pub(crate) status: String,
}

impl JiraDialog {
    #[must_use]
    pub(crate) fn new_ready(
        jql: String,
        page_size: u16,
        export_dir: PathBuf,
        base_url: String,
    ) -> Self {
        let mut dialog = Self::new_setup(String::new(), jql.clone(), export_dir);
        dialog.base_url = base_url;
        dialog.page_size = page_size;
        dialog.queue(JiraRequest::Fetch { jql, page_size });
        dialog
    }

    #[must_use]
    pub(crate) fn new_setup(message: String, jql: String, export_dir: PathBuf) -> Self {
        let setup_error = (!message.is_empty()).then(|| message.clone());
        Self {
            generation: Arc::new(()),
            latest: JiraRequestId::default(),
            setup_error,
            issues: Vec::new(),
            user: None,
            base_url: String::new(),
            jql,
            page_size: 50,
            export_dir,
            filter: String::new(),
            cursor: 0,
            marks: BTreeSet::new(),
            mode: JiraMode::Browse,
            busy: None,
            queued: None,
            status: message,
        }
    }

    pub(crate) fn queue(&mut self, request: JiraRequest) {
        self.setup_error = None;
        let id = self.latest.next();
        self.latest = id;
        self.busy = Some(request.busy());
        request.busy().label().clone_into(&mut self.status);
        self.queued = Some((id, request));
    }

    pub(crate) fn take_queued(&mut self) -> Option<(JiraRequestId, JiraRequest)> {
        self.queued.take()
    }

    #[must_use]
    pub(crate) const fn awaits(&self, id: JiraRequestId) -> bool {
        self.latest.0 == id.0
    }

    #[must_use]
    pub(crate) const fn is_busy(&self) -> bool {
        self.busy.is_some() || self.queued.is_some()
    }

    pub(crate) fn apply_reply(
        &mut self,
        reply: JiraReply,
    ) -> Result<Option<ExportReport>, JiraError> {
        self.busy = None;
        match reply {
            JiraReply::Fetched(result) => {
                let listing = result?;
                let shown = listing.issues.len();
                let total = listing.total;
                self.user = Some(listing.user);
                self.issues = listing.issues;
                self.jql = listing.jql;
                self.base_url = listing.base_url;
                self.cursor = 0;
                self.marks.clear();
                self.status = format!("Loaded {shown} of {total} issue(s)");
                Ok(None)
            }
            JiraReply::Exported(result) => {
                let report = result?;
                self.status.clone_from(&report.summary);
                Ok(Some(report))
            }
        }
    }
}
