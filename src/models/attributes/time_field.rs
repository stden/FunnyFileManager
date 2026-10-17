//! Purpose: The attributes dialog's time fields: the text they offer and
//! accept, and the times they ask for.

use std::time::SystemTime;

/// One time field: the text it opened with, and the text it shows now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeField {
    pub initial: String,
    pub current: String,
}

impl TimeField {
    pub(super) fn offering(text: String) -> Self {
        Self {
            initial: text.clone(),
            current: text,
        }
    }

    /// The time this field asks for, parsed.
    ///
    /// `None` both for a blank field and for one left as it opened, like an
    /// unmoved box: the offered text is the first target's own time, and
    /// with Process subfolders on it would otherwise land on everything
    /// inside.
    pub(super) fn request(&self, field: &'static str) -> Result<Option<SystemTime>, InvalidTime> {
        if self.current == self.initial {
            return Ok(None);
        }
        parse_time(field, &self.current)
    }
}

/// The times the user asked for; `None` leaves that time as it is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestedTimes {
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

/// A time field that does not read as a date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTime {
    pub field: &'static str,
    pub text: String,
}

impl std::fmt::Display for InvalidTime {
    #[expect(
        clippy::use_debug,
        reason = "Debug quotes and escapes the typed text, which is how the refusal has always shown it"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:?} is not a time ({TIME_FORMAT_HINT})",
            self.field, self.text
        )
    }
}

/// How the time fields read and are written: one format, so what the dialog
/// offers is exactly what it accepts back.
pub const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const TIME_FORMAT_HINT: &str = "YYYY-MM-DD HH:MM:SS";

/// A timestamp as the field shows it, in local time.
#[must_use]
pub fn format_time(at: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format(TIME_FORMAT)
        .to_string()
}

/// The field's text back into a timestamp; blank means "leave it".
#[expect(
    clippy::map_err_ignore,
    reason = "the refusal names the field, the text and the format; chrono's reason adds nothing to act on"
)]
pub(super) fn parse_time(
    field: &'static str,
    text: &str,
) -> Result<Option<SystemTime>, InvalidTime> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let refused = || InvalidTime {
        field,
        text: text.to_owned(),
    };
    let naive = chrono::NaiveDateTime::parse_from_str(text, TIME_FORMAT).map_err(|_| refused())?;
    let local = naive
        .and_local_timezone(chrono::Local)
        .single()
        .ok_or_else(refused)?;
    Ok(Some(SystemTime::from(local)))
}
