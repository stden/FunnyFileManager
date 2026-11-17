//! Purpose: Pin the explorer's two effects: opening the dialog from
//! `databases.yaml`, and running one request on its own connection.

mod execute;
mod open;
pub(super) mod pg_wire;
mod server;
