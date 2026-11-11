//! Purpose: The Cloudflare DNS vocabulary the adapter and the dialog share: zones, records,
//! the requests and answers of the background job, and the errors. Nothing here does I/O.

mod change;
mod request;
mod types;

pub use change::{full_name, Change, NewRecord, RecordUpdate};
pub use request::{Busy, CurrentHost, Reply, Request};
pub use types::{
    CloudflareAccess, CloudflareError, Proxy, Record, RecordId, Ttl, Zone, ZoneId, TOKEN_VARIABLE,
};
