//! Purpose: `WinSCP`'s URL encoding undone — every `.netbox` value is decoded
//! before it is checked.

use crate::sessions::import::SkipReason;

/// `WinSCP`'s URL encoding undone: `%XX` is the byte `XX`, and the bytes
/// must be UTF-8. A `%` without two hex digits after it is an error rather
/// than a literal, so a damaged value is never half-decoded.
#[expect(
    clippy::map_err_ignore,
    reason = "the offset of the bad UTF-8 sequence means nothing to the user; the reason names the setting"
)]
pub(super) fn decode(text: &str, setting: &'static str) -> Result<String, SkipReason> {
    let bad = || SkipReason::BadEncoding(setting);
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let [high, low, tail @ ..] = rest else {
            return Err(bad());
        };
        bytes.push(hex_byte(*high, *low).ok_or_else(bad)?);
        rest = tail;
    }
    String::from_utf8(bytes).map_err(|_| bad())
}

/// The byte two hex digits spell.
fn hex_byte(high: u8, low: u8) -> Option<u8> {
    let digit = |byte: u8| char::from(byte).to_digit(16);
    u8::try_from(digit(high)? * 16 + digit(low)?).ok()
}
