//! Purpose: The Cloudflare DNS dialog's state and the moves that need no request: the cursors,
//! the focus, and the single request that may be outstanding at a time.

use std::sync::Arc;

use super::{Busy, Change, Record, RecordForm, Request, Zone, ZoneId};

/// Numbers the requests of one dialog, so an answer to an older one is dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestId(u64);

impl RequestId {
    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Which list the arrow keys move in while the records are browsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Zones,
    Records,
}

/// The dialog's one screen at a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Browse,
    Edit(Box<RecordForm>),
    /// The form travels with the change, so a refused write returns to it.
    Confirm(Box<RecordForm>, Box<Change>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Browse,
    Edit,
    Confirm,
}

impl View {
    pub(crate) const fn kind(&self) -> ViewKind {
        match self {
            Self::Browse => ViewKind::Browse,
            Self::Edit(_) => ViewKind::Edit,
            Self::Confirm(..) => ViewKind::Confirm,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CloudflareDnsPanel {
    /// Identifies this opening of the dialog: an answer for an earlier opening is dropped.
    pub(crate) generation: Arc<()>,
    pub(crate) zones: Vec<Zone>,
    pub(crate) zone: usize,
    pub(crate) records: Vec<Record>,
    pub(crate) record: usize,
    pub(crate) focus: Focus,
    pub(crate) view: View,
    pub(crate) status: String,
    /// Set from the moment a request is queued until its answer is applied.
    pub(crate) busy: Option<Busy>,
    pub(crate) latest: RequestId,
    pub(crate) queued: Option<Request>,
}

impl CloudflareDnsPanel {
    pub(crate) fn new() -> Self {
        Self {
            generation: Arc::new(()),
            zones: Vec::new(),
            zone: 0,
            records: Vec::new(),
            record: 0,
            focus: Focus::Zones,
            view: View::Browse,
            status: String::new(),
            busy: None,
            latest: RequestId::default(),
            queued: None,
        }
    }

    pub(crate) fn selected_zone(&self) -> Option<&Zone> {
        self.zones.get(self.zone)
    }

    pub(crate) fn selected_record(&self) -> Option<&Record> {
        self.records.get(self.record)
    }

    /// Queues `request`, unless another one is still outstanding. Returns whether it was queued.
    pub(crate) fn enqueue(&mut self, request: Request) -> bool {
        if self.busy.is_some() {
            return false;
        }
        self.latest = self.latest.next();
        self.busy = Some(request.busy());
        self.queued = Some(request);
        true
    }

    /// The queued request and the id its answer has to carry.
    pub(crate) fn take_queued(&mut self) -> Option<(RequestId, Request)> {
        let request = self.queued.take()?;
        Some((self.latest, request))
    }

    /// Whether `id` is the request this dialog waits for.
    pub(crate) fn awaits(&self, id: RequestId) -> bool {
        self.busy.is_some() && self.latest == id
    }

    pub(crate) fn settle(&mut self) {
        self.busy = None;
    }

    /// Moves the cursor of the focused list one row. Returns the zone whose records
    /// must be loaded when the zone itself moved.
    pub(crate) fn step(&mut self, forward: bool) -> Option<ZoneId> {
        match self.focus {
            Focus::Records => {
                self.record = moved(self.record, self.records.len(), forward);
                None
            }
            Focus::Zones => {
                let zone = moved(self.zone, self.zones.len(), forward);
                (zone != self.zone)
                    .then(|| self.select_zone(zone))
                    .flatten()
            }
        }
    }

    pub(crate) const fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Zones => Focus::Records,
            Focus::Records => Focus::Zones,
        };
    }

    /// Selects the zone at `index` and forgets the records of the one left.
    /// Returns the id whose records must be loaded.
    pub(crate) fn select_zone(&mut self, index: usize) -> Option<ZoneId> {
        self.zone = index;
        self.records.clear();
        self.record = 0;
        self.selected_zone().map(|zone| zone.id.clone())
    }

    pub(crate) fn zone_index(&self, id: &ZoneId) -> Option<usize> {
        self.zones.iter().position(|zone| zone.id == *id)
    }

    /// Replaces the zone list, keeping the selected zone when it is still there.
    /// Returns the zone whose records must now be loaded.
    pub(crate) fn set_zones(&mut self, zones: Vec<Zone>) -> Option<ZoneId> {
        let kept = self.selected_zone().map(|zone| zone.id.clone());
        self.zones = zones;
        self.zone = kept.and_then(|id| self.zone_index(&id)).unwrap_or(0);
        self.records.clear();
        self.record = 0;
        self.selected_zone().map(|zone| zone.id.clone())
    }

    /// Replaces the record list, keeping the cursor on a row that still exists.
    pub(crate) fn set_records(&mut self, records: Vec<Record>) {
        self.record = self.record.min(records.len().saturating_sub(1));
        self.records = records;
    }
}

/// The index one step from `current` in a list of `len` rows, held at the ends.
fn moved(current: usize, len: usize, forward: bool) -> usize {
    if forward {
        current.saturating_add(1).min(len.saturating_sub(1))
    } else {
        current.saturating_sub(1)
    }
}
