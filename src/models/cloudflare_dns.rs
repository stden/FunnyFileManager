//! Purpose: The Cloudflare DNS dialog's state: zones, records, the record form, the confirmation,
//! and the requests the background job answers. Nothing here does I/O.

mod form;
mod panel;

#[cfg(test)]
pub use crate::values::cloudflare_dns::RecordId;
pub use crate::values::cloudflare_dns::{
    Busy, Change, CloudflareAccess, CloudflareError, CurrentHost, NewRecord, Proxy, Record,
    RecordUpdate, Reply, Request, Ttl, Zone, ZoneId,
};
pub use form::{FormField, RecordForm, Target};
pub use panel::CloudflareDnsPanel;
pub use panel::{Focus, RequestId, View, ViewKind};

#[cfg(test)]
mod tests;
