//! How much an ifcZIP entry may inflate to.

/// The decompression budget of an ifcZIP read.
///
/// Two limits bound the inflated model document, and whichever is smaller
/// applies:
///
/// - **absolute**: at most [`Self::max_bytes`] bytes;
/// - **ratio**: at most [`Self::max_ratio`] times the entry's compressed
///   size, once the document is larger than [`Self::ratio_floor`] bytes (a
///   small, highly regular file may legitimately compress better than any
///   ratio a real model reaches).
///
/// Both are checked against the entry's declared size before inflating and
/// against the bytes actually inflated while inflating, so an archive that
/// understates its sizes is refused too, with
/// [`IfcZipError::BudgetExceeded`](super::IfcZipError::BudgetExceeded).
///
/// The defaults admit any model a 64-bit reader can hold in memory below
/// 4 GiB, and a ratio of 200: STEP compresses about 5 to 10 times and
/// ifcXML 10 to 20 times, while Deflate alone reaches about 1032 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Budget {
    /// The largest inflated document, in bytes.
    pub max_bytes: u64,
    /// The largest inflated-to-compressed size ratio.
    pub max_ratio: u64,
    /// The inflated size below which the ratio is not checked.
    pub ratio_floor: u64,
}

impl Budget {
    /// The default budget: 4 GiB, a ratio of 200 above 1 MiB.
    pub const DEFAULT: Self = Self {
        max_bytes: 4 << 30,
        max_ratio: 200,
        ratio_floor: 1 << 20,
    };

    /// This budget with a different absolute limit.
    #[must_use]
    pub const fn max_bytes(mut self, bytes: u64) -> Self {
        self.max_bytes = bytes;
        self
    }

    /// This budget with a different ratio limit.
    #[must_use]
    pub const fn max_ratio(mut self, ratio: u64) -> Self {
        self.max_ratio = ratio;
        self
    }

    /// This budget with a different size below which the ratio is not
    /// checked.
    #[must_use]
    pub const fn ratio_floor(mut self, bytes: u64) -> Self {
        self.ratio_floor = bytes;
        self
    }

    /// The most an entry of `compressed` bytes may inflate to, and which
    /// limit sets it.
    pub(crate) fn allowance(&self, compressed: u64) -> (u64, Limit) {
        let by_ratio = compressed
            .saturating_mul(self.max_ratio)
            .max(self.ratio_floor);
        if by_ratio < self.max_bytes {
            (
                by_ratio,
                Limit::Ratio {
                    ratio: self.max_ratio,
                    compressed,
                },
            )
        } else {
            (self.max_bytes, Limit::Bytes(self.max_bytes))
        }
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The limit of a [`Budget`] an entry exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Limit {
    /// The absolute limit, in bytes.
    Bytes(u64),
    /// The ratio limit, for an entry of `compressed` bytes.
    Ratio {
        /// The ratio.
        ratio: u64,
        /// The entry's compressed size.
        compressed: u64,
    },
}

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes(bytes) => write!(f, "the absolute budget of {bytes} bytes"),
            Self::Ratio { ratio, compressed } => write!(
                f,
                "the ratio budget of {ratio}:1 for {compressed} compressed bytes"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_smaller_limit_applies() {
        let budget = Budget::DEFAULT.max_bytes(1000).max_ratio(10).ratio_floor(0);
        assert_eq!(
            budget.allowance(50),
            (
                500,
                Limit::Ratio {
                    ratio: 10,
                    compressed: 50
                }
            )
        );
        assert_eq!(budget.allowance(500), (1000, Limit::Bytes(1000)));
        let floored = budget.ratio_floor(800);
        assert_eq!(floored.allowance(1).0, 800, "the floor admits small files");
        assert_eq!(
            Budget::DEFAULT.allowance(u64::MAX),
            (4 << 30, Limit::Bytes(4 << 30)),
            "the ratio saturates instead of overflowing"
        );
    }
}
