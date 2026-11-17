//! Purpose: What the dialog asks the background job to do, and the answers it applies.

use super::{Change, CloudflareError, Record, Zone, ZoneId};

/// One request to Cloudflare, or to the machine it runs on for the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Zones,
    Records(ZoneId),
    Host,
    Write(Change),
}

/// What the dialog is waiting for, shown as its loading line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Busy {
    Zones,
    Records,
    Host,
    Saving,
}

impl Request {
    pub(crate) const fn busy(&self) -> Busy {
        match self {
            Self::Zones => Busy::Zones,
            Self::Records(_) => Busy::Records,
            Self::Host => Busy::Host,
            Self::Write(_) => Busy::Saving,
        }
    }

    /// The answer to this request when it could not be made.
    pub(crate) fn fail(self, error: CloudflareError) -> Reply {
        match self {
            Self::Zones => Reply::Zones(Err(error)),
            Self::Records(zone) => Reply::Records(zone, Err(error)),
            Self::Host => Reply::Host(Err(error)),
            Self::Write(_) => Reply::Written(Err(error)),
        }
    }
}

/// The answer to one [`Request`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Zones(Result<Vec<Zone>, CloudflareError>),
    Records(ZoneId, Result<Vec<Record>, CloudflareError>),
    Host(Result<CurrentHost, CloudflareError>),
    Written(Result<Record, CloudflareError>),
}

/// This machine as a new record would name it. The address is `Err` when it
/// could not be found, and the reason is what the form says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentHost {
    pub(crate) hostname: String,
    pub(crate) ipv4: Result<String, CloudflareError>,
}
