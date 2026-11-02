//! Purpose: The Cloudflare DNS dialog's state: zones, records, the record form, the confirmation,
//! and the requests the background job answers. Nothing here does I/O.

mod change;
mod form;
mod panel;
mod request;
mod types;

pub use change::{Change, NewRecord, RecordUpdate};
pub use form::{FormField, RecordForm, Target};
pub use panel::CloudflareDnsPanel;
pub use panel::{Focus, RequestId, View, ViewKind};
pub use request::{Busy, CurrentHost, Reply, Request};
pub use types::{
    CloudflareAccess, CloudflareError, Proxy, Record, RecordId, Ttl, Zone, ZoneId, TOKEN_VARIABLE,
};

#[cfg(test)]
mod tests;
