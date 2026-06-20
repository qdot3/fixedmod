//! Fast modular arithmetic with a fixed modulus.
//!
//! This crate provides [`Modulus`], which computes metadata for a given modulus
//! so that modular multiplication, reduction, and divisibility checks
//! can be performed without integer division.
//!
//! Constructing a [`Modulus`] is relatively costly (it involves one division),
//! so create it once for a given modulus and reuse it across many operations.
//!
//! # Example
//!
//! ```
//! use core::num::NonZeroU32;
//! use fixedmod::Modulus;
//!
//! let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
//!
//! assert_eq!(modulus.mul_mod(5, 6), 2);   // (5 * 6) % 7
//! assert_eq!(modulus.reduce32(10), 3);    // 10 % 7
//! assert_eq!(modulus.reduce64(u64::MAX), (u64::MAX % 7) as u32);
//! assert!(modulus.is_divisible(14));         // 14 % 7 == 0
//! ```
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_qualifications)]
#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]
#![no_std]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

use core::num::NonZeroU32;

/// Precomputed metadata for fast modular arithmetic.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Modulus {
    value: NonZeroU32,
    magic: u64,
    mask: u32,
    offset: u32,
}

impl Modulus {
    /// Create a new instance.
    ///
    /// # Usage
    ///
    /// This constructor involves a division and is relatively costly.
    /// Construct it once and reuse the resulting `Modulus`.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    /// ```
    #[must_use]
    pub const fn new(value: NonZeroU32) -> Self {
        let (div, rem) = {
            let m = value.get() as u64;
            (u64::MAX / m, u64::MAX % m)
        };

        // `ceil(2^64 / m)`
        let magic = div.wrapping_add(1);
        // `2^64 % m`
        let offset = {
            let rem = rem as u32 + 1;
            if rem == value.get() {
                0
            } else {
                rem
            }
        };
        // Equivalent to `if m == 1 { 0 } else { !0 }`, but branchless and faster.
        // When `m == 1`, `magic` overflows to `0` during the computation above.
        // `mask` corrects for this case.
        let mask = 0_u32.wrapping_sub((value.get() > 1) as u32);

        Self {
            value,
            magic,
            mask,
            offset,
        }
    }

    /// Returns the modulus specified at creation.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    /// assert_eq!(modulus.value().get(), 7)
    /// ```
    #[must_use]
    pub const fn value(&self) -> NonZeroU32 {
        self.value
    }

    /// Performs modular multiplication `a * b % m` without division.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert_eq!(modulus.mul_mod(5, 6), 30 % 7);
    /// assert_eq!(
    ///     modulus.mul_mod(u32::MAX, u32::MAX),
    ///     ((u32::MAX as u64 * u32::MAX as u64) % 7) as u32
    /// );
    /// ```
    #[must_use]
    pub const fn mul_mod(&self, a: u32, b: u32) -> u32 {
        self.reduce64_by_nontrivial(a as u64 * b as u64) & self.mask
    }

    /// Performs modular exponentiation `a.pow(exp) % m` without overflow or division.
    ///
    /// # Time complexity
    ///
    /// O(log `exp`)
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let m = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert_eq!(m.pow_mod(3, 4), 81 % 7);  // 3^4 = 81
    /// assert_eq!(m.pow_mod(5, 0), 1);       // a^0 == 1
    /// ```
    #[must_use]
    pub const fn pow_mod(&self, mut a: u32, mut exp: u32) -> u32 {
        // Binary exponentiation (square-and-multiply).

        let mut result = 1;
        while exp > 0 {
            if exp & 1 == 1 {
                result = self.reduce64_by_nontrivial(result as u64 * a as u64);
            }
            a = self.reduce64_by_nontrivial(a as u64 * a as u64);
            exp >>= 1;
        }

        result & self.mask
    }

    /// Computes the modular multiplicative inverse of `a`.
    ///
    /// When `m == 1`, every integer is congruent to `0` modulo `m`,
    /// so this always returns `Ok(0)`.
    ///
    /// # Errors
    ///
    /// If the modular inverse does not exist, this returns `gcd(a, m)`.
    ///
    /// # Time complexity
    ///
    /// O(log `m`)
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let m = Modulus::new(NonZeroU32::new(7).unwrap());
    /// assert_eq!(m.inv(3), Ok(5)); // 3 * 5 = 15 ≡ 1 (mod 7)
    ///
    /// let m = Modulus::new(NonZeroU32::new(6).unwrap());
    /// assert_eq!(m.inv(4), Err(2)); // gcd(4, 6) == 2
    ///
    /// let m = Modulus::new(NonZeroU32::new(1).unwrap());
    /// assert_eq!(m.inv(123), Ok(0));
    /// ```
    pub const fn inv(&self, a: u32) -> Result<u32, u32> {
        #![allow(clippy::many_single_char_names)]
        #![allow(clippy::cast_possible_truncation)]
        #![allow(clippy::cast_sign_loss)]
        // Extended Euclidean algorithm

        // invariant: x a = u, y a = v (mod m)
        let mut u = self.reduce32(a) as i64;
        let mut v = self.value.get() as i64;
        let [mut x, mut y] = [1, 0];

        while u > 0 {
            let (div, rem) = (v / u, v % u);
            (u, v) = (rem, u);
            (x, y) = (y - div * x, x);

            debug_assert!((v * y).abs() <= self.value.get() as i64);
        }
        debug_assert!(u == 0 && v > 0);

        // v = gcd(a, m)
        if v != 1 {
            return Err(v as u32);
        }

        debug_assert!(x * y <= 0);
        y += if y < 0 { self.value.get() as i64 } else { 0 };
        Ok(y as u32)
    }

    /// Performs reduction `a % m` without division.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert_eq!(modulus.reduce32(10), 3);
    /// assert_eq!(modulus.reduce32(14), 0);
    /// ```
    #[must_use]
    pub const fn reduce32(&self, a: u32) -> u32 {
        // Lemire's fast remainder algorithm.
        // Reuses the same `magic` as Barrett reduction, but needs one fewer multiplication.

        // Fractional part of `a / m`
        // When `m == 1`, `magic == 0`, so this is `0`, and `rem` below is also `0`.
        let frac = self.magic.wrapping_mul(a as u64);
        #[allow(clippy::cast_possible_truncation)]
        let rem = ((frac as u128 * self.value.get() as u128) >> 64) as u32;

        rem
    }

    /// Performs reduction `a % m` without division.
    ///
    /// When `a` fits in `u32`, use [`reduce32`](Self::reduce32), which is faster.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert_eq!(modulus.reduce64(u64::MAX), (u64::MAX % 7) as u32);
    /// assert_eq!(modulus.reduce64(u64::MAX / 7 * 7), 0);
    /// ```
    #[must_use]
    pub const fn reduce64(&self, a: u64) -> u32 {
        // When `m == 1`, `magic` is `0` and `rem` is meaningless.
        // `mask` filters this case out, forcing the result to `0`.
        self.reduce64_by_nontrivial(a) & self.mask
    }

    /// # Precondition
    ///
    /// `m > 1`
    #[allow(clippy::inline_always)]
    #[inline(always)]
    const fn reduce64_by_nontrivial(&self, a: u64) -> u32 {
        // Computes `a - a / m * m` for `m > 1`

        // `quot` approximates `a / m`, but may be `1` larger than the true value.
        let quot = ((a as u128 * self.magic as u128) >> 64) as u64;
        // An underflow here means `quot` overshot the true value by `1`.
        // We correct for this by adding `modulus` back.
        let (rem, borrow) = a.overflowing_sub(quot * self.value.get() as u64);
        #[allow(clippy::cast_possible_truncation)]
        let rem = (rem as u32).wrapping_add(if borrow { self.value.get() } else { 0 });

        rem
    }

    /// Performs reduction `a.rem_euclid(m)` without division.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert_eq!(modulus.reduce64_signed(-10), 4);
    /// assert_eq!(modulus.reduce64_signed(10), 3);
    /// ```
    pub const fn reduce64_signed(&self, a: i64) -> u32 {
        // `0 <= rem < m`.
        let rem = self.reduce64_by_nontrivial(a.cast_unsigned());
        // `-m < rem < m`.
        let (rem, borrow) = rem.overflowing_sub(
            // when `a < 0`, `a.cast_unsigned() == a + 2^64`
            if a.is_negative() { self.offset } else { 0 },
        );
        // `0 <= rem < m`
        let rem = rem.wrapping_add(if borrow { self.value.get() } else { 0 });

        rem & self.mask
    }

    /// Returns `true` if `a` is divisible by `m`.
    ///
    /// # Example
    ///
    /// ```
    /// use core::num::NonZeroU32;
    /// use fixedmod::Modulus;
    ///
    /// let modulus = Modulus::new(NonZeroU32::new(7).unwrap());
    ///
    /// assert!(modulus.is_divisible(14));
    /// assert!(!modulus.is_divisible(10));
    /// ```
    #[must_use]
    pub const fn is_divisible(&self, a: u32) -> bool {
        // Lemire's divisibility test

        // Fractional part of `a / m`.
        // When `m == 1`, `magic == 0`, so this is always `0`.
        let frac = self.magic.wrapping_mul(a as u64);

        // is_divisible iff `frac < magic`.
        // `wrapping_sub` handles the case of `m == 1`, where RHS is `u64::MAX`.
        frac <= self.magic.wrapping_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use proptest::prelude::*;

    use crate::Modulus;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 15))]
        #[test]
        fn reduce32(a: u32, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            assert_eq!(modulus.reduce32(a), a % m.get())
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 15))]
        #[test]
        fn reduce64(a: u64, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            assert_eq!(modulus.reduce64(a), (a % m.get() as u64) as u32)
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 15))]
        #[test]
        fn reduce64_signed(a: i64, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            assert_eq!(modulus.reduce64_signed(a), a.rem_euclid(m.get() as i64) as u32)
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 15))]
        #[test]
        fn is_divisible(a: u32, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            assert_eq!(modulus.is_divisible(a), a % m.get() == 0)
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 10))]
        #[test]
        fn pow_mod(a: u32, exp: u16, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            let mut expected = modulus.reduce32(1);
            for _ in 0..exp {
                expected = modulus.mul_mod(expected, a);
            }

            assert_eq!(modulus.pow_mod(a, exp as u32), expected);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 15))]
        #[test]
        fn inv(a: u32, m: NonZeroU32) {
            let modulus = Modulus::new(m);

            match modulus.inv(a) {
                Ok(inv) => assert_eq!(modulus.mul_mod(a, inv), 1 % m.get()),
                Err(gcd) => {
                    assert!(a % gcd == 0 && m.get() % gcd == 0);

                    let m = m.get() / gcd;
                    let a = a / gcd;
                    let modulus = Modulus::new(NonZeroU32::new(m).unwrap());
                    let inv = modulus.inv(a).unwrap();
                    assert_eq!(modulus.mul_mod(a, inv), 1 % m);
                }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1 << 13))]
        #[test]
        fn division_by_1(a: u32, b: u64, exp: u32) {
            let modulus = Modulus::new(NonZeroU32::MIN);

            assert_eq!(modulus.reduce32(a), 0);
            assert_eq!(modulus.reduce64(b), 0);
            assert_eq!(modulus.reduce64_signed(b.cast_signed()), 0);
            assert!(modulus.is_divisible(a));
            assert_eq!(modulus.pow_mod(a, exp), 0);
            assert_eq!(modulus.inv(a), Ok(0));
        }
    }

    #[test]
    fn zero() {
        let modulus = Modulus::new(NonZeroU32::MIN);

        assert_eq!(modulus.pow_mod(0, 0), 0);
    }
}
