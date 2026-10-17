//! Purpose: The run-queue load average over one, five and fifteen minutes,
//! each in hundredths.

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
