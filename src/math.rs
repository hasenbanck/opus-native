use std::mem::size_of;

/// Commonly used logarithms on integer primitives.
pub(crate) trait Log: Sized + Copy + PartialOrd {
    fn leading_zeros_impl(self) -> u32;
    fn zero(self) -> Self;

    /// The minimum number of bits required to store a positive integer in binary, or 0 for a non-positive integer.
    #[inline(always)]
    fn ilog(self) -> u32 {
        (size_of::<Self>() * 8) as u32 - self.leading_zeros_impl()
    }

    /// Log base 2. Self needs to be > 0.
    #[inline(always)]
    fn log2(self) -> u32 {
        debug_assert!(self > self.zero());
        self.ilog() - 1
    }
}

impl Log for i16 {
    #[inline(always)]
    fn leading_zeros_impl(self) -> u32 {
        self.leading_zeros()
    }

    #[inline(always)]
    fn zero(self) -> Self {
        0
    }
}

impl Log for u32 {
    #[inline(always)]
    fn leading_zeros_impl(self) -> u32 {
        self.leading_zeros()
    }

    #[inline(always)]
    fn zero(self) -> Self {
        0
    }
}

impl Log for usize {
    #[inline(always)]
    fn leading_zeros_impl(self) -> u32 {
        self.leading_zeros()
    }

    #[inline(always)]
    fn zero(self) -> Self {
        0
    }
}