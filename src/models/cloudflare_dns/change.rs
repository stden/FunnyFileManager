//! Purpose: A validated edit of one record, and the exact text its confirmation view shows.

use super::{Proxy, Record, Ttl, Zone};

/// The only record type the dialog creates.
const NEW_KIND: &str = "A";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRecord {
    pub(crate) zone: Zone,
    /// Relative to the zone, or the full name when it already ends with the zone.
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) ttl: Ttl,
    pub(crate) proxy: Proxy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordUpdate {
    pub(crate) zone: Zone,
    /// The record as it was when the form opened: the old values.
    pub(crate) record: Record,
    pub(crate) content: String,
    pub(crate) ttl: Ttl,
    pub(crate) proxy: Proxy,
}

/// The fields an update changes, and to what. `None` leaves a field alone, so
/// the request carries only what really changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    pub(crate) content: Option<String>,
    pub(crate) ttl: Option<Ttl>,
    pub(crate) proxy: Option<Proxy>,
}

impl Changed {
    pub(crate) const fn is_empty(&self) -> bool {
        self.content.is_none() && self.ttl.is_none() && self.proxy.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Create(NewRecord),
    Update(RecordUpdate),
}

impl Change {
    /// The confirmation text: the zone, the record, then each value that
    /// changes, as `old -> new` for an update.
    pub(crate) fn lines(&self) -> Vec<String> {
        match self {
            Self::Create(new) => vec![
                format!("Zone: {}", new.zone.name),
                format!(
                    "New record: {} ({NEW_KIND})",
                    full_name(&new.name, &new.zone.name)
                ),
                format!("content: {}", new.content),
                format!("TTL: {}", new.ttl.label()),
                format!("proxied: {}", new.proxy.label()),
            ],
            Self::Update(update) => update.lines(),
        }
    }
}

impl RecordUpdate {
    /// The fields whose new value differs from the record's.
    pub(crate) fn changed(&self) -> Changed {
        Changed {
            content: (self.content != self.record.content).then(|| self.content.clone()),
            ttl: (self.ttl != self.record.ttl).then_some(self.ttl),
            proxy: (self.proxy != self.record.proxy && self.proxy != Proxy::Unsupported)
                .then_some(self.proxy),
        }
    }

    fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Zone: {}", self.zone.name),
            format!("Record: {} ({})", self.record.name, self.record.kind),
        ];
        let changed = self.changed();
        if let Some(content) = &changed.content {
            lines.push(format!("content: {} -> {content}", self.record.content));
        }
        if let Some(ttl) = changed.ttl {
            lines.push(format!(
                "TTL: {} -> {}",
                self.record.ttl.label(),
                ttl.label()
            ));
        }
        if let Some(proxy) = changed.proxy {
            lines.push(format!(
                "proxied: {} -> {}",
                self.record.proxy.label(),
                proxy.label()
            ));
        }
        lines
    }
}

/// The name a record has once the API reads it: a relative name sits under the
/// zone, and a name that already ends with the zone is the full name.
pub fn full_name(name: &str, zone: &str) -> String {
    if name == zone || name.ends_with(&format!(".{zone}")) {
        name.to_owned()
    } else {
        format!("{name}.{zone}")
    }
}
