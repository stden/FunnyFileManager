//! Purpose: The units the task list measures in — shares in hundredths of a
//! percent, memory in bytes, load in hundredths — each with the one checked
//! conversion from what a process source reports.
//!
//! All arithmetic is integer and checked or saturating: release builds abort
//! on a panic and do not check overflow, so a wrapped value would be drawn as
//! a plausible wrong number instead of failing loudly.

use std::num::NonZeroU128;

/// A share in hundredths of a percent: `FULL` is 100 %.
///
/// A process's CPU share can exceed `FULL`: a process busy on four cores is at
/// 400 %, up to the core count times `FULL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percent(u32);

impl Percent {
    /// 100 %.
    pub const FULL: Self = Self(10_000);

    #[must_use]
    pub const fn from_hundredths(hundredths: u32) -> Self {
        Self(hundredths)
    }

    #[must_use]
    pub const fn hundredths(self) -> u32 {
        self.0
    }

    /// `part` of `whole`, rounded half up to a hundredth of a percent and
    /// saturated at `u32::MAX` hundredths; `None` when `whole` is zero, since
    /// nothing is a share of nothing.
    #[must_use]
    pub fn ratio(part: u128, whole: u128) -> Option<Self> {
        NonZeroU128::new(whole).map(|whole| Self::of(part, whole))
    }

    /// [`Percent::ratio`] for a whole already known not to be zero.
    ///
    /// The whole multiples of `whole` in `part` and what is left over are
    /// scaled apart, so only a share too large to hold saturates, never a
    /// large operand.
    #[must_use]
    pub fn of(part: u128, whole: NonZeroU128) -> Self {
        let whole = whole.get();
        let hundredths = (part / whole)
            .saturating_mul(u128::from(Self::FULL.0))
            .saturating_add(u128::from(Self::fraction(part % whole, whole)));
        Self(u32::try_from(hundredths).unwrap_or(u32::MAX))
    }

    /// `below / whole` in hundredths of a percent, rounded half up, for
    /// `below` less than `whole`: `0..=FULL`.
    ///
    /// `below * FULL` can overflow `u128`, so the product is never held whole.
    /// It is built from the top bit of `FULL` down, doubling and adding as in
    /// Horner's rule, and kept as a count of `whole`s plus a remainder below
    /// `whole`: the count never exceeds `FULL` and nothing overflows.
    fn fraction(below: u128, whole: u128) -> u32 {
        let full = Self::FULL.0;
        let mut product = (0, 0);
        for bit in (0..u32::BITS - full.leading_zeros()).rev() {
            let (wholes, remainder) = product;
            product = add_below((wholes * 2, remainder), remainder, whole);
            if full & (1 << bit) != 0 {
                product = add_below(product, below, whole);
            }
        }
        let (wholes, remainder) = product;
        wholes + u32::from(remainder >= whole - remainder)
    }

    /// This share, or `ceiling` when it is larger.
    #[must_use]
    pub fn at_most(self, ceiling: Self) -> Self {
        self.min(ceiling)
    }
}

/// `(wholes, remainder)` plus `addend`, carrying into `wholes` when the
/// remainder reaches `whole`. `remainder` and `addend` are both below `whole`,
/// so at most one `whole` is carried and no sum overflows.
fn add_below((wholes, remainder): (u32, u128), addend: u128, whole: u128) -> (u32, u128) {
    let room = whole - addend;
    if remainder >= room {
        (wholes + 1, remainder - room)
    } else {
        (wholes, remainder + addend)
    }
}

/// How much CPU a process used since the previous sample.
///
/// `Pending` sorts before every measured share, so an unmeasured process never
/// sits between two measured ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuUsage {
    /// Not measurable yet: the first sample, a process new since the last one,
    /// or no time elapsed. Shown as `-`, never as a made-up `0.0`.
    Pending,
    Measured(Percent),
}

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

/// A load average in hundredths: `52` is `0.52`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Load(u32);

impl Load {
    #[must_use]
    pub const fn from_hundredths(hundredths: u32) -> Self {
        Self(hundredths)
    }

    #[must_use]
    pub const fn hundredths(self) -> u32 {
        self.0
    }
}

/// The run-queue load averaged over the last one, five and fifteen minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadAverage {
    one: Load,
    five: Load,
    fifteen: Load,
}

impl LoadAverage {
    #[must_use]
    pub const fn new(one: Load, five: Load, fifteen: Load) -> Self {
        Self { one, five, fifteen }
    }

    #[must_use]
    pub const fn one(self) -> Load {
        self.one
    }

    #[must_use]
    pub const fn five(self) -> Load {
        self.five
    }

    #[must_use]
    pub const fn fifteen(self) -> Load {
        self.fifteen
    }
}
