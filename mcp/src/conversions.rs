//! Purpose: Checked numeric conversions behind the tool reports: human-readable
//! sizes, client-supplied counts, S3 object sizes and byte tallies.

pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if size >= GB {
        format!("{} GB", two_decimals(size, GB))
    } else if size >= MB {
        format!("{} MB", two_decimals(size, MB))
    } else if size >= KB {
        format!("{} KB", two_decimals(size, KB))
    } else {
        format!("{size} B")
    }
}

/// `size / unit` to two decimals, rounded half to even as `{:.2}` rounds.
///
/// Integer maths in `u128`: exact for every `u64`, where an `f64` quotient
/// would lose the low bits of any size above 2^53.
fn two_decimals(size: u64, unit: u64) -> String {
    let unit = u128::from(unit);
    let scaled = u128::from(size) * 100;
    let (mut hundredths, remainder) = (scaled / unit, scaled % unit);
    if remainder * 2 > unit || (remainder * 2 == unit && hundredths % 2 == 1) {
        hundredths += 1;
    }
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

/// A client-supplied `u64` count as a `usize`, saturating where `usize` is
/// narrower: a line offset or limit past `usize::MAX` means "all of them".
pub fn saturating_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// A `usize` count or length as a `u64`: lossless on every supported target,
/// saturating on one whose `usize` is wider.
pub fn saturating_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// An S3 object size as bytes. The SDK models it as `i64`; a missing or
/// negative size reports as zero rather than wrapping to 16 EiB.
pub fn object_size(size: Option<i64>) -> u64 {
    size.and_then(|bytes| u64::try_from(bytes).ok())
        .unwrap_or(0)
}

/// How many times `needle` occurs in `haystack`.
#[expect(
    clippy::naive_bytecount,
    reason = "a SIMD byte-count crate is not worth a dependency for line-ending tallies"
)]
pub fn count_byte(haystack: &[u8], needle: u8) -> usize {
    haystack.iter().filter(|byte| **byte == needle).count()
}
