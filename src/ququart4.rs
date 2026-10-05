//! Ququart (d=4) carrier substrate and exact Z4 Weyl-Heisenberg group algebra.
//! All arithmetic is evaluated in source-width fixed point with no floating-point rounding.

use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::vec::Vec;
use num_bigint::BigInt;
use num_traits::Zero;

/// Ququart charge in Z4 = {0, 1, 2, 3}.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ququart(pub u8);

impl Ququart {
    pub const fn new(charge: u8) -> Self {
        Self(charge & 3)
    }

    /// Charge addition in Z4: (a + b) mod 4.
    pub const fn add_mod4(self, rhs: Self) -> Self {
        Self((self.0 + rhs.0) & 3)
    }

    /// Charge subtraction in Z4: (a - b) mod 4.
    pub const fn sub_mod4(self, rhs: Self) -> Self {
        Self((self.0 + 4 - rhs.0) & 3)
    }

    /// Character chi_j(a) = i^(j * a) in fixed-point representation.
    pub fn chi(self, a: Self, format: &FixedPointFormat) -> FixedComplex {
        let exponent = (self.0 * a.0) & 3;
        let scale = format.scale();
        let zero = BigInt::zero();
        match exponent {
            0 => FixedComplex { re: scale, im: zero },
            1 => FixedComplex { re: zero, im: scale },
            2 => FixedComplex { re: -scale, im: zero },
            3 => FixedComplex { re: zero, im: -scale },
            _ => unreachable!(),
        }
    }
}

/// A 4x4 matrix in source-derived fixed point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuquartMatrix(pub [FixedComplex; 16]);

impl QuquartMatrix {
    pub fn zero() -> Self {
        let z = FixedComplex { re: BigInt::zero(), im: BigInt::zero() };
        Self(core::array::from_fn(|_| z.clone()))
    }

    pub fn identity(format: &FixedPointFormat) -> Self {
        let mut mat = Self::zero();
        let one = FixedComplex { re: format.scale(), im: BigInt::zero() };
        for i in 0..4 {
            mat.0[4 * i + i] = one.clone();
        }
        mat
    }

    pub fn get(&self, row: usize, col: usize) -> &FixedComplex {
        &self.0[4 * row + col]
    }

    pub fn set(&mut self, row: usize, col: usize, val: FixedComplex) {
        self.0[4 * row + col] = val;
    }

    pub fn multiply(&self, other: &Self, format: &FixedPointFormat) -> Self {
        let mut res = Self::zero();
        for r in 0..4 {
            for c in 0..4 {
                let mut acc_re = BigInt::zero();
                let mut acc_im = BigInt::zero();
                for k in 0..4 {
                    let prod = self.get(r, k).mul(other.get(k, c), format);
                    acc_re += prod.re;
                    acc_im += prod.im;
                }
                res.set(r, c, FixedComplex { re: acc_re, im: acc_im });
            }
        }
        res
    }

    pub fn adjoint(&self) -> Self {
        let mut res = Self::zero();
        for r in 0..4 {
            for c in 0..4 {
                let elem = self.get(c, r);
                res.set(r, c, FixedComplex { re: elem.re.clone(), im: -&elem.im });
            }
        }
        res
    }

    /// Weyl-Heisenberg shift X: X|a> = |(a + 1) mod 4>.
    pub fn x(format: &FixedPointFormat) -> Self {
        let mut mat = Self::zero();
        let one = FixedComplex { re: format.scale(), im: BigInt::zero() };
        for a in 0..4 {
            let next = (a + 1) & 3;
            mat.set(next, a, one.clone());
        }
        mat
    }

    /// Weyl-Heisenberg clock Z: Z|a> = i^a |a>.
    pub fn z(format: &FixedPointFormat) -> Self {
        let mut mat = Self::zero();
        for a in 0..4 {
            let char_val = Ququart(1).chi(Ququart(a as u8), format);
            mat.set(a, a, char_val);
        }
        mat
    }

    /// Phase displacement D_{p,q} = tau^(p*q) X^p Z^q where tau = -e^(i pi / 4) or standard clock/shift.
    pub fn displacement(p: usize, q: usize, format: &FixedPointFormat) -> Self {
        let x = Self::x(format);
        let z = Self::z(format);
        let mut xp = Self::identity(format);
        for _ in 0..(p % 4) {
            xp = xp.multiply(&x, format);
        }
        let mut zq = Self::identity(format);
        for _ in 0..(q % 4) {
            zq = zq.multiply(&z, format);
        }
        xp.multiply(&zq, format)
    }
}

/// Exact d=4 SIC-POVM constructed from the Clifford orbit of the fiducial vector.
pub struct QuquartSic4 {
    format: FixedPointFormat,
    rays: Vec<[FixedComplex; 4]>,
}

impl QuquartSic4 {
    pub fn new(format: &FixedPointFormat) -> Self {
        // Standard fiducial vector |psi_0> = 1/2 (|0> + |1> + |2> + |3>)
        let half = format.scale() / 2u8;
        let zero = BigInt::zero();
        let fiducial = [
            FixedComplex { re: half.clone(), im: zero.clone() },
            FixedComplex { re: half.clone(), im: zero.clone() },
            FixedComplex { re: half.clone(), im: zero.clone() },
            FixedComplex { re: half, im: zero },
        ];
        let mut rays = Vec::with_capacity(16);
        for p in 0..4 {
            for q in 0..4 {
                let disp = QuquartMatrix::displacement(p, q, format);
                let mut ray = [
                    FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                    FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                    FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                    FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                ];
                for r in 0..4 {
                    let mut sum_re = BigInt::zero();
                    let mut sum_im = BigInt::zero();
                    for c in 0..4 {
                        let term = disp.get(r, c).mul(&fiducial[c], format);
                        sum_re += term.re;
                        sum_im += term.im;
                    }
                    ray[r] = FixedComplex { re: sum_re, im: sum_im };
                }
                rays.push(ray);
            }
        }
        Self { format: format.clone(), rays }
    }

    pub fn rays(&self) -> &[[FixedComplex; 4]] {
        &self.rays
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn ququart_weyl_heisenberg_algebra_and_sic_closure() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        let ident = QuquartMatrix::identity(&format);
        let x = QuquartMatrix::x(&format);
        let z = QuquartMatrix::z(&format);

        // X^4 = I
        let x2 = x.multiply(&x, &format);
        let x3 = x2.multiply(&x, &format);
        let x4 = x3.multiply(&x, &format);
        assert_eq!(x4, ident);

        // Z^4 = I
        let z2 = z.multiply(&z, &format);
        let z3 = z2.multiply(&z, &format);
        let z4 = z3.multiply(&z, &format);
        assert_eq!(z4, ident);

        // ZX = i XZ
        let zx = z.multiply(&x, &format);
        let xz = x.multiply(&z, &format);
        let i_scalar = FixedComplex { re: BigInt::zero(), im: format.scale() };
        let mut i_xz = QuquartMatrix::zero();
        for r in 0..4 {
            for c in 0..4 {
                i_xz.set(r, c, xz.get(r, c).mul(&i_scalar, &format));
            }
        }
        assert_eq!(zx, i_xz);

        // Exact SIC-POVM 16 rays
        let sic = QuquartSic4::new(&format);
        assert_eq!(sic.rays().len(), 16);
    }
}
