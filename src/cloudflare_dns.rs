//! Purpose: Cloudflare DNS reads and writes over bounded HTTPS. The token is sent only to the API;
//! the host lookup reaches the trace page without it.

mod call;
mod client;
mod codec;
mod host;
mod list;
mod token;
mod validate;
mod write;
mod zone;

#[cfg(test)]
mod tests;

use crate::models::cloudflare_dns::{CloudflareError, Reply, Request};

pub use call::Transport;
#[cfg(test)]
pub use call::{Call, Method};

pub use validate::{content, proxy, record_name, ttl};
pub use zone::{choose, relative_name};

/// Runs one request from the background job, on the network.
pub fn fetch(request: Request) -> Reply {
    match request {
        Request::Host => Reply::Host(host::detect()),
        request => match client::Client::from_environment() {
            Ok(mut client) => execute(&mut client, request),
            Err(error) => request.fail(error),
        },
    }
}

/// Answers one request through `transport`.
pub fn execute(transport: &mut dyn Transport, request: Request) -> Reply {
    match request {
        Request::Zones => Reply::Zones(list::zones(transport)),
        Request::Records(zone) => {
            let records = list::records(transport, &zone);
            Reply::Records(zone, records)
        }
        Request::Write(change) => Reply::Written(write::apply(transport, &change)),
        Request::Host => Reply::Host(host::detect()),
    }
}

/// The answer to a request while the live network is switched off.
pub fn unavailable(request: Request) -> Reply {
    request.fail(CloudflareError::Disabled)
}
