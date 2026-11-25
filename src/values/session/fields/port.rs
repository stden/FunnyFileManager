//! Purpose: A session's TCP port, 1 to 65535: the protocols' defaults, parsing
//! the typed text, and the `serde` conversions.

use std::fmt;
use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};

use super::{Field, InvalidField, Problem};

/// A TCP port: 1 to 65535.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Port(NonZeroU16);

impl Port {
    /// The SSH port, which SFTP and SCP use: 22.
    pub const SSH: Self = Self(NonZeroU16::MIN.saturating_add(22 - 1));
    /// The FTP control port: 21.
    pub const FTP: Self = Self(NonZeroU16::MIN.saturating_add(21 - 1));

    pub fn new(number: u16) -> Result<Self, InvalidField> {
        NonZeroU16::new(number)
            .map(Self)
            .ok_or(InvalidField::new(Field::Port, Problem::NotAPort))
    }

    /// A port as typed: digits only, surrounding blanks dropped.
    #[expect(
        clippy::map_err_ignore,
        reason = "the refusal names the field and says it is not a port; the integer error adds nothing to act on"
    )]
    pub fn parse(text: &str) -> Result<Self, InvalidField> {
        let text = text.trim();
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(InvalidField::new(Field::Port, Problem::NotAPort));
        }
        text.parse::<u16>()
            .map_err(|_| InvalidField::new(Field::Port, Problem::NotAPort))
            .and_then(Self::new)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u16> for Port {
    type Error = InvalidField;

    fn try_from(number: u16) -> Result<Self, Self::Error> {
        Self::new(number)
    }
}

impl From<Port> for u16 {
    fn from(port: Port) -> Self {
        port.get()
    }
}
