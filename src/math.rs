use std::mem::size_of;

/// Commonly used logarithms on integer primitives.
pub(crate) trait Log: Sized + Copy + PartialOrd {
    fn leading_zeros_impl(self) -> u32;
    fn zero(self) -> Self;

    /// Log base 2 + 1.
    #[inline(always)]
    fn log2p1(self) -> u32 {
        (size_of::<Self>() * 8) as u32 - self.leading_zeros_impl()
    }

    /// Log base 2. Self needs to be > 0.
    #[inline(always)]
    fn log2(self) -> u32 {
        debug_assert!(self > self.zero());
        self.log2p1() - 1
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
