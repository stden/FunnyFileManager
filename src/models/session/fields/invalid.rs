//! Purpose: The one error every session value check reports in: which field,
//! and what is wrong with it — never the refused value itself.

use std::fmt;

/// Which value of a session a check is about, named as the dialog names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Host,
    Port,
    User,
    RemoteDir,
    KeyFile,
}

impl Field {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Host => "Host",
            Self::Port => "Port",
            Self::User => "User",
            Self::RemoteDir => "Directory",
            Self::KeyFile => "Key file",
        }
    }
}

/// What is wrong with a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    Empty,
    TooLong {
        limit: usize,
    },
    ControlCharacter,
    /// A character this value cannot hold; printed escaped.
    Character(char),
    /// Starts with `-`, which a command line would take for an option.
    LeadingDash,
    NotAbsolute,
    /// A `.` or `..` part in a path.
    DotSegment,
    /// Not a whole number from 1 to 65535.
    NotAPort,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("must not be empty"),
            Self::TooLong { limit } => write!(f, "must be at most {limit} characters"),
            Self::ControlCharacter => f.write_str("must not hold control characters"),
            Self::Character(ch) => write!(f, "must not hold '{}'", ch.escape_debug()),
            Self::LeadingDash => f.write_str("must not start with '-'"),
            Self::NotAbsolute => f.write_str("must start with '/'"),
            Self::DotSegment => f.write_str("must not hold '.' or '..' parts"),
            Self::NotAPort => f.write_str("must be a number from 1 to 65535"),
        }
    }
}

/// A value that failed its check: which one, and why. Carries no part of
/// the value, so reporting it can never echo what was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidField {
    pub field: Field,
    pub problem: Problem,
}

impl InvalidField {
    #[must_use]
    pub const fn new(field: Field, problem: Problem) -> Self {
        Self { field, problem }
    }
}

impl fmt::Display for InvalidField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field.label(), self.problem)
    }
}

impl std::error::Error for InvalidField {}
