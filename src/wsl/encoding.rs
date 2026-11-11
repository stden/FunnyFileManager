//! Purpose: Decoding what `wsl.exe` printed: UTF-16 in either byte order,
//! with or without a BOM, or UTF-8.

/// Two bytes of UTF-16 in one byte order, as one code unit.
type Utf16Decoder = fn([u8; 2]) -> u16;

/// The one message for output that is neither UTF-8 nor UTF-16.
const INVALID_TEXT: &str = "WSL returned invalid text encoding";

/// WSL versions emit UTF-16LE (with or without a BOM) or UTF-8 on redirected pipes.
#[expect(
    clippy::map_err_ignore,
    reason = "the offset of the bad UTF-8 sequence means nothing to the user; the fixed message does"
)]
pub(super) fn decode_text(bytes: &[u8], truncated: bool) -> Result<String, String> {
    let (bytes, encoding): (&[u8], Option<Utf16Decoder>) =
        if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
            (bytes, Some(u16::from_le_bytes))
        } else if let Some(bytes) = bytes.strip_prefix(&[0xfe, 0xff]) {
            (bytes, Some(u16::from_be_bytes))
        } else if looks_like_bomless_utf16le(bytes, truncated) {
            (bytes, Some(u16::from_le_bytes))
        } else {
            (
                bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
                None,
            )
        };
    let text = if let Some(decode) = encoding {
        decode_utf16(bytes, truncated, decode)?
    } else if truncated {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|_| INVALID_TEXT.to_owned())?
    };
    Ok(text.trim_matches('\0').to_owned())
}

fn looks_like_bomless_utf16le(bytes: &[u8], truncated: bool) -> bool {
    if !bytes.len().is_multiple_of(2) && !truncated {
        return false;
    }
    let pairs = bytes.as_chunks::<2>().0;
    let ascii_utf16 = pairs.iter().filter(|&&[lo, hi]| lo != 0 && hi == 0).count();
    if ascii_utf16 == 0 {
        return false;
    }
    let bare_newline = pairs
        .iter()
        .any(|&[lo, hi]| (lo == b'\n' && hi != 0) || hi == b'\n');
    !bare_newline && ascii_utf16 * 2 >= pairs.len()
}

#[expect(
    clippy::map_err_ignore,
    reason = "the offset of the lone surrogate means nothing to the user; the fixed message does"
)]
fn decode_utf16(bytes: &[u8], truncated: bool, decode: Utf16Decoder) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) && !truncated {
        return Err(INVALID_TEXT.to_owned());
    }
    let words: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(decode)
        .collect();
    if truncated {
        Ok(String::from_utf16_lossy(&words))
    } else {
        String::from_utf16(&words).map_err(|_| INVALID_TEXT.to_owned())
    }
}

#[cfg(test)]
mod tests;
