//! Purpose: The identities, record values and errors the Cloudflare DNS dialog shows and changes.

/// The environment variable that holds the API token. Only its name is ever shown.
pub const TOKEN_VARIABLE: &str = "CLOUDFLARE_API_TOKEN";

/// A zone identifier, as the API returns it. Kept apart from [`RecordId`] so a
/// record id can never be sent where a zone id belongs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneId(String);

impl ZoneId {
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A DNS record identifier, as the API returns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordId(String);

impl RecordId {
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    pub(crate) id: ZoneId,
    pub(crate) name: String,
}

/// A record's time to live. The API spells automatic as `1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ttl {
    Automatic,
    Seconds(u32),
}

impl Ttl {
    pub(crate) fn label(self) -> String {
        match self {
            Self::Automatic => "automatic".to_owned(),
            Self::Seconds(seconds) => seconds.to_string(),
        }
    }
}

/// Whether Cloudflare proxies a record. Only some record types can be proxied,
/// and the API omits the field for the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proxy {
    Off,
    On,
    Unsupported,
}

impl Proxy {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Unsupported => "n/a",
        }
    }

    pub(crate) const fn toggled(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Off,
            Self::Unsupported => Self::Unsupported,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub(crate) id: RecordId,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) ttl: Ttl,
    pub(crate) proxy: Proxy,
}

/// Whether the background job may reach Cloudflare. A test or a replayed
/// session keeps it [`Offline`](Self::Offline): nothing is read or sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloudflareAccess {
    #[default]
    Offline,
    Network,
}

/// Why a request failed. Every message is safe to show: it never carries the
/// token or a response body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloudflareError {
    MissingToken,
    Disabled,
    AccessDenied,
    RateLimited,
    Network,
    Status(u16),
    InvalidResponse,
    Limit,
    /// The first error of a Cloudflare error envelope, with its code.
    Api {
        code: u64,
        message: String,
    },
    NoZone(String),
    HostName,
    Invalid(String),
    NoChange,
    Worker,
}

impl CloudflareError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::MissingToken => format!("set {TOKEN_VARIABLE}"),
            Self::Disabled => "Unavailable in replay or isolated sessions".to_owned(),
            Self::AccessDenied => "Access denied: check the API token's permissions".to_owned(),
            Self::RateLimited => "Rate limited by Cloudflare; retry later".to_owned(),
            Self::Network => "Network request to Cloudflare failed".to_owned(),
            Self::Status(code) => format!("Cloudflare answered HTTP {code}"),
            Self::InvalidResponse => "Unsupported Cloudflare response".to_owned(),
            Self::Limit => "Cloudflare response exceeded its page or size limit".to_owned(),
            Self::Api { code, message } => format!("Cloudflare error {code}: {message}"),
            Self::NoZone(hostname) => {
                format!("No zone in this account matches hostname {hostname}")
            }
            Self::HostName => "The hostname could not be read".to_owned(),
            Self::Invalid(text) => text.clone(),
            Self::NoChange => "No changes to apply".to_owned(),
            Self::Worker => "Cloudflare worker failed".to_owned(),
        }
    }
}
