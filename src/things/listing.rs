//! Purpose: What one read of the catalog returned: its machines in name order,
//! and the total the server reports, parsed from the body shapes Things sends.

use serde::Deserialize;

use super::{Machine, ThingsError};

/// The machines one read returned, sorted by name, and the total the server
/// reports for the whole catalog when it reports one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub machines: Vec<Machine>,
    pub total: Option<usize>,
}

impl Listing {
    /// `machines` in name order, ignoring case. Names that differ only in case
    /// keep the order the server gave them.
    #[must_use]
    pub fn new(mut machines: Vec<Machine>, total: Option<usize>) -> Self {
        machines.sort_by_cached_key(|machine| machine.name.to_lowercase());
        Self { machines, total }
    }
}

/// The three body shapes: a bare array, `{success, data, total_count}`, and
/// `{machines}`. Fields not named here are ignored.
#[derive(Deserialize)]
#[serde(untagged)]
enum Body {
    Bare(Vec<Machine>),
    Data {
        success: Option<bool>,
        data: Vec<Machine>,
        total_count: Option<usize>,
    },
    Named {
        machines: Vec<Machine>,
    },
}

impl Body {
    fn into_listing(self) -> Result<Listing, ThingsError> {
        match self {
            Self::Bare(machines) | Self::Named { machines } => Ok(Listing::new(machines, None)),
            Self::Data {
                success: Some(false),
                ..
            } => Err(ThingsError::Response),
            Self::Data {
                data, total_count, ..
            } => Ok(Listing::new(data, total_count)),
        }
    }
}

/// The listing a response body holds, or [`ThingsError::Response`] when it
/// holds none.
pub fn parse(body: &[u8]) -> Result<Listing, ThingsError> {
    let body: Body = serde_json::from_slice(body).map_err(|_json_error| ThingsError::Response)?;
    body.into_listing()
}
