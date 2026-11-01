//! Purpose: A file's size in bytes, its megabyte figure, and the rounded
//! human-readable form the size column shows.

/// The units a human-readable size is scaled by, largest first.
const UNITS: [(u64, &str); 3] = [(1 << 30, "GB"), (1 << 20, "MB"), (1 << 10, "KB")];

/// The unit a `size` of bytes is shown in, or `None` below a kibibyte.
fn unit_of(size: u64) -> Option<(u64, &'static str)> {
    UNITS.iter().copied().find(|&(unit, _)| size >= unit)
}

/// A file's size in bytes.
///
/// Named because a bare `u64` in this program is just as likely to be a line
/// count, a byte offset or a cell budget — and because the megabyte conversion
/// and the human-readable rendering belong with the number rather than in free
/// functions anyone can call with any integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSize(u64);

impl FileSize {
    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }

    /// The size in megabytes — the unit the status bar reports in.
    ///
    /// Whole mebibytes and the remainder each convert to `f64` exactly, so the
    /// result is exact; it saturates at `u32::MAX` MiB (4 PiB).
    #[must_use]
    pub fn megabytes(self) -> f64 {
        const MIB: u32 = 1 << 20;
        let Ok(whole) = u32::try_from(self.0 / u64::from(MIB)) else {
            return f64::from(u32::MAX);
        };
        // Below `MIB`, so it always fits; the fallback is never taken.
        let rest = u32::try_from(self.0 % u64::from(MIB)).unwrap_or(0);
        f64::from(whole) + f64::from(rest) / f64::from(MIB)
    }

    /// The size to one decimal place, in the units `Display` uses: "93.1 KB".
    /// Below a kibibyte it is the bare byte count.
    #[must_use]
    pub fn one_decimal(self) -> String {
        let Some((unit, name)) = unit_of(self.0) else {
            return format!("{} B", self.0);
        };
        let tenths = divide_rounded(u128::from(self.0) * 10, u128::from(unit));
        format!("{}.{} {name}", tenths / 10, tenths % 10)
    }
}

impl std::fmt::Display for FileSize {
    /// Human-readable, as the panel's size column shows it: "1.50 KB",
    /// "3.20 MB", or a bare byte count below a kibibyte.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.0;
        let Some((unit, name)) = unit_of(size) else {
            return write!(f, "{size} B");
        };
        let hundredths = divide_rounded(u128::from(size) * 100, u128::from(unit));
        write!(f, "{}.{:02} {name}", hundredths / 100, hundredths % 100)
    }
}

/// `numerator / denominator` to the nearest integer, ties to even.
///
/// That is the rule `format!("{:.2}", x)` applies to an exact value, so a size
/// scaled with integers prints the digits the float formatting printed, without
/// `u64 as f64` rounding the byte count above 2^53. `denominator` is non-zero.
pub fn divide_rounded(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let to_next = denominator - remainder;
    let rounds_up = remainder > to_next || (remainder == to_next && quotient % 2 == 1);
    quotient + u128::from(rounds_up)
}
