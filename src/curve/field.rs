#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as arch;
use std::fmt::Debug;

/// adc computes out <- a + b + carry, outputting a new carry.
///
/// `carry` must be 0, or 1. The return value will satisfy this constraint
#[inline]
pub fn adc(carry: u8, a: u64, b: u64, out: &mut u64) -> u8 {
    #[cfg(target_arch = "x86_64")]
    {
        // Using this intrinsic is perfectly safe
        unsafe { arch::_addcarry_u64(carry, a, b, out) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // The largest result is 2 * (2^64 - 1) + 1 = 2^65 - 1, which needs exactly 65 bits
        // Hence, we use u128. Hopefully, Rust will realize that we don't really want to use
        // 128 bit operations, but rather want to use an `adc` instruction, or whatever equivalent
        // our ISA has, and insert that instead.
        let full_res = u128::from(a) + u128::from(b) + u128::from(carry);
        *out = full_res as u64;
        (full_res >> 64) as u8
    }
}

/// N is the number of limbs in our representation.
const N: usize = 4;

#[derive(Clone, Copy)]
// Only implement equality for tests. This is to avoid the temptation to introduce
// a timing leak through equality comparison.
#[cfg_attr(test, derive(PartialEq))]
pub struct Fp([u64; N]);

impl Debug for Fp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fp(0x")?;
        for (i, x) in self.0.iter().rev().enumerate() {
            if i > 0 {
                write!(f, "_")?;
            }
            write!(f, "{:08X}", x)?;
        }
        write!(f, ")")
    }
}

impl Fp {
    pub fn add(&mut self, other: Self) {
        let mut carry: u8 = 0;
        // Let's have confidence in Rust's ability to unroll this loop.
        for i in 0..4 {
            // Each intermediate result may generate up to 65 bits of output.
            // We need to daisy-chain the carries together, to get the right result.
            carry = adc(carry, self.0[i], other.0[i], &mut self.0[i]);
        }
    }

    pub fn constant() -> Self {
        Self([0xFF, 0xFF, 0xFF, 0xFF])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug() {
        dbg!(Fp([0, 1, 2, 3]));
        assert!(false)
    }
}
