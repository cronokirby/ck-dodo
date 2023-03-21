use std::fmt::Debug;

/// N is the number of limbs in our representation.
const N: usize = 4;

#[derive(Clone, Copy, PartialEq)]
struct Fp([u64; N]);

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug() {
        dbg!(Fp([0, 1, 2, 3]));
        assert!(false)
    }
}
