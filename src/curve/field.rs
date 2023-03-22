#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as arch;
use std::{fmt::Debug, ops::MulAssign};

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
pub struct Fp {
    limbs: [u64; N],
}

impl Debug for Fp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fp(0x")?;
        for (i, x) in self.limbs.iter().rev().enumerate() {
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
            carry = adc(carry, self.limbs[i], other.limbs[i], &mut self.limbs[i]);
        }
    }

    pub fn constant() -> Self {
        Self {
            limbs: [0xFF, 0xFF, 0xFF, 0xFF],
        }
    }
}

use std::arch::asm;

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
impl MulAssign for Fp {
    fn mul_assign(&mut self, other: Fp) {
        let mut out = [0u64; 2 * N];
        for i in 0..N {
            unsafe {
                asm!(
                    "test rax, rax",

                    "mulx {6}, {5}, [{7}]",
                    "adcx {0}, {5}",
                    "adox {1}, {6}",

                    "mulx {6}, {5}, [{7} + 1]",
                    "adcx {1}, {5}",
                    "adox {2}, {6}",

                    "mulx {6}, {5}, [{7} + 2]",
                    "adcx {2}, {5}",
                    "adox {3}, {6}",

                    "mulx {6}, {5}, [{7} + 3]",
                    "adcx {3}, {5}",
                    "adox {4}, {6}",

                    "adc {4}, 0",

                    out(reg) out[i],
                    out(reg) out[i + 1],
                    out(reg) out[i + 2],
                    out(reg) out[i + 3],
                    out(reg) out[i + 4],
                    out(reg) _,
                    out(reg) _,
                    in(reg) &other.limbs,
                    in("rdx") self.limbs[i],
                );
            }
        }

        let mut carry = 0u64;
        for i in 0..4 {
            let full_res = u128::from(carry) + u128::from(out[i]) + 38 * u128::from(out[4 + i]);
            self.limbs[i] = full_res as u64;
            carry = (full_res >> 64) as u64;
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl MulAssign for Fp {
    fn mul_assign(&mut self, other: Fp) {
        // You can treat both of these functions as macros. They just exist to avoid
        // repeating this logic multiple times.

        // This calculates u:v = a * b, and then adds u:v to r2:r1:r0
        #[inline(always)]
        fn multiply_in(a: u64, b: u64, r0: &mut u64, r1: &mut u64, r2: &mut u64) {
            let uv = u128::from(a) * u128::from(b);
            let mut carry = 0;
            carry = adc(carry, uv as u64, *r0, r0);
            carry = adc(carry, (uv >> 64) as u64, *r1, r1);
            *r2 += u64::from(carry);
        }

        // Given r2:r1:r0, this sets limb = r0, and then shifts to get 0:r2:r1
        #[inline(always)]
        fn propagate(limb: &mut u64, r0: &mut u64, r1: &mut u64, r2: &mut u64) {
            *limb = *r0;
            *r0 = *r1;
            *r1 = *r2;
            *r2 = 0;
        }

        // We need 8 limbs to hold the full multiplication result, so we need an
        // extra buffer. By using the extra buffer to store the low limbs,
        // we can clobber self with the high limbs, without overwriting any limbs
        // necessary for further calculations.
        let mut low = Fp { limbs: [0u64; 4] };

        // This is essentially a 192 bit number
        let mut r0 = 0u64;
        let mut r1 = 0u64;
        let mut r2 = 0u64;

        // This is an unrolling of big loop that looks like:
        //    for k = 0..6
        //      for i in 0..3, j in 0..3 with i + j = k:
        //        multiply_in(self[i], other[j])
        //      propagate(out[k])
        //    propagate(out[7])
        //
        // The rough idea here is to add in all of the factors that contribute to a given
        // limb of the output, adding in carries from the previous step, and then propagating
        // a carry to the next step.

        multiply_in(self.limbs[0], other.limbs[0], &mut r0, &mut r1, &mut r2);
        propagate(&mut low.limbs[0], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[0], other.limbs[1], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[1], other.limbs[0], &mut r0, &mut r1, &mut r2);
        propagate(&mut low.limbs[1], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[0], other.limbs[2], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[1], other.limbs[1], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[2], other.limbs[0], &mut r0, &mut r1, &mut r2);
        propagate(&mut low.limbs[2], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[0], other.limbs[3], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[1], other.limbs[2], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[2], other.limbs[1], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[3], other.limbs[0], &mut r0, &mut r1, &mut r2);
        propagate(&mut low.limbs[3], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[1], other.limbs[3], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[2], other.limbs[2], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[3], other.limbs[1], &mut r0, &mut r1, &mut r2);
        propagate(&mut self.limbs[0], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[2], other.limbs[3], &mut r0, &mut r1, &mut r2);
        multiply_in(self.limbs[3], other.limbs[2], &mut r0, &mut r1, &mut r2);
        propagate(&mut self.limbs[1], &mut r0, &mut r1, &mut r2);

        multiply_in(self.limbs[3], other.limbs[3], &mut r0, &mut r1, &mut r2);
        propagate(&mut self.limbs[2], &mut r0, &mut r1, &mut r2);

        self.limbs[3] = r0;

        // At this point, we've multiplied things out, and have:
        //     self⋅2²⁵⁶ + low
        // Observe that 2²⁵⁶ = 2⋅(2²⁵⁵ - 19) + 38, so mod P, we have:
        //     low + 38⋅self
        // All that's left is to multiply self by 38, and then add in low
        let mut carry = 0u64;
        for i in 0..4 {
            let full_res =
                u128::from(carry) + u128::from(low.limbs[i]) + 38 * u128::from(self.limbs[i]);
            self.limbs[i] = full_res as u64;
            carry = (full_res >> 64) as u64;
        }
        //self.reduce_after_scaling(carry);
    }
}
