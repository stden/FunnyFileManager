//! Strict file codecs: decode without replacement and encode before touching disk.
use std::io;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextEncoding {
    #[default]
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Windows1251,
    Oem866,
    Windows1252,
}

impl TextEncoding {
    pub const ALL: [Self; 7] = [
        Self::Utf8,
        Self::Utf8Bom,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Windows1251,
        Self::Oem866,
        Self::Windows1252,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Windows1251 => "Windows-1251",
            Self::Oem866 => "OEM-866",
            Self::Windows1252 => "Windows-1252",
        }
    }

    /// The code page F8 switches to, as Far's OEM/ANSI toggle does: OEM-866
    /// becomes Windows-1251, and any other encoding becomes OEM-866 first.
    #[must_use]
    pub const fn oem_ansi_toggle(self) -> Self {
        match self {
            Self::Oem866 => Self::Windows1251,
            _ => Self::Oem866,
        }
    }

    pub fn detect(bytes: &[u8]) -> Self {
        if bytes.starts_with(b"\xef\xbb\xbf") {
            Self::Utf8Bom
        } else if bytes.starts_with(b"\xff\xfe") {
            Self::Utf16Le
        } else if bytes.starts_with(b"\xfe\xff") {
            Self::Utf16Be
        } else if std::str::from_utf8(bytes).is_ok() {
            Self::Utf8
        } else {
            Self::Windows1251
        }
    }

    pub fn decode(self, bytes: &[u8]) -> io::Result<String> {
        let bytes = bytes.strip_prefix(self.bom()).unwrap_or(bytes);
        self.codec()
            .decode_without_bom_handling_and_without_replacement(bytes)
            .map(std::borrow::Cow::into_owned)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid {} text", self.label()),
                )
            })
    }

    /// Decode for reading only: a byte this encoding cannot map becomes U+FFFD
    /// instead of failing the whole file. Never for text that will be written
    /// back — the editor uses [`Self::decode`], which refuses rather than let a
    /// save replace those bytes with replacement marks.
    pub fn decode_lossy(self, bytes: &[u8]) -> String {
        self.decode_lossy_cow(bytes).into_owned()
    }

    /// Borrow valid UTF-8 directly without allocating an intermediate `String`.
    pub(crate) fn decode_lossy_cow(self, bytes: &[u8]) -> std::borrow::Cow<'_, str> {
        let bytes = bytes.strip_prefix(self.bom()).unwrap_or(bytes);
        self.codec().decode_without_bom_handling(bytes).0
    }

    pub fn encode(self, text: &str) -> io::Result<Vec<u8>> {
        let mut bytes = self.bom().to_vec();
        if matches!(self, Self::Utf16Le | Self::Utf16Be) {
            bytes.extend(utf16_bytes(text, self == Self::Utf16Le));
        } else {
            let (encoded, _, errors) = self.codec().encode(text);
            if errors {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Text contains characters unavailable in {}", self.label()),
                ));
            }
            bytes.extend_from_slice(&encoded);
        }
        Ok(bytes)
    }

    const fn codec(self) -> &'static encoding_rs::Encoding {
        match self {
            Self::Utf8 | Self::Utf8Bom => encoding_rs::UTF_8,
            Self::Utf16Le => encoding_rs::UTF_16LE,
            Self::Utf16Be => encoding_rs::UTF_16BE,
            Self::Windows1251 => encoding_rs::WINDOWS_1251,
            Self::Oem866 => encoding_rs::IBM866,
            Self::Windows1252 => encoding_rs::WINDOWS_1252,
        }
    }

    const fn bom(self) -> &'static [u8] {
        match self {
            Self::Utf8Bom => b"\xef\xbb\xbf",
            Self::Utf16Le => b"\xff\xfe",
            Self::Utf16Be => b"\xfe\xff",
            Self::Utf8 | Self::Windows1251 | Self::Oem866 | Self::Windows1252 => b"",
        }
    }
}

/// Whether `bytes` are binary rather than text.
///
/// A NUL is git's heuristic. Behind a UTF-16 byte-order mark a NUL byte is
/// only the high half of an ASCII character, so there the test is a NUL
/// *character*: a code unit of two zero bytes.
pub fn is_binary(bytes: &[u8]) -> bool {
    let behind_utf16_bom = [TextEncoding::Utf16Le, TextEncoding::Utf16Be]
        .into_iter()
        .find_map(|encoding| bytes.strip_prefix(encoding.bom()));
    if let Some(text) = behind_utf16_bom {
        let (units, _) = text.as_chunks::<2>();
        return units.contains(&[0, 0]);
    }
    memchr::memchr(0, bytes).is_some()
}

#[cfg(test)]
mod tests;

fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
    text.encode_utf16()
        .flat_map(|unit| {
            if little_endian {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            }
        })
        .collect()
}
