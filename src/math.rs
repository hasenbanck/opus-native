use std::mem::size_of;

use num_traits::{Num, PrimInt, Zero};

/// Commonly used logarithms on integer primitives.
pub(crate) trait Log: PrimInt + Zero {
    /// The minimum number of bits required to store a positive integer in binary, or 0 for a non-positive integer.
    #[inline(always)]
    fn ilog(self) -> u32 {
        (size_of::<Self>() * 8) as u32 - self.leading_zeros()
    }

    /// Log base 2. Self needs to be > 0.
    #[inline(always)]
    fn log2(self) -> u32 {
        debug_assert!(!self.is_zero());
        self.ilog() - 1
    }
}

impl Log for u32 {}
