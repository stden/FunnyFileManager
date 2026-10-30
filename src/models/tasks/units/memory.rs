//! Purpose: An amount of memory in bytes, with the checked conversions from
//! the kibibytes and pages a process source reports.

/// An amount of memory in bytes.
///
/// Kept apart from `FileSize`, whose display and meaning are a file's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryBytes(u64);

impl MemoryBytes {
    const KIB: u64 = 1024;

    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// `kib` kibibytes — what `/proc/meminfo` calls `kB` — or `None` when the
    /// byte count would not fit.
    #[must_use]
    pub const fn from_kib(kib: u64) -> Option<Self> {
        match kib.checked_mul(Self::KIB) {
            Some(bytes) => Some(Self(bytes)),
            None => None,
        }
    }

    /// `count` pages of `page` bytes each, or `None` when the byte count would
    /// not fit.
    #[must_use]
    pub const fn from_pages(count: u64, page: u64) -> Option<Self> {
        match count.checked_mul(page) {
            Some(bytes) => Some(Self(bytes)),
            None => None,
        }
    }

    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }

    /// This amount plus `other`, or `u64::MAX` bytes when the sum overflows.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// This amount less `other`, or zero when `other` is larger.
    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}
