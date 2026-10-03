//! Source-derived fixed-precision evaluation of local Fibonacci braids.
//! Only a two-dimensional gate operator is held here, independent of the
//! global factorization register. This is not a complete carrier readout.
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::recycled_carrier::BraidTarget;
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

#[derive(Clone, Debug)]
pub struct LocalMatrix(pub [FixedComplex; 4]);

impl LocalMatrix {
    pub fn identity(format: &FixedPointFormat) -> Self {
        let zero = FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        };
        let one = FixedComplex {
            re: format.scale(),
            im: BigInt::zero(),
        };
        Self([one.clone(), zero.clone(), zero, one])
    }
    pub fn multiply(&self, other: &Self, format: &FixedPointFormat) -> Self {
        Self(core::array::from_fn(|index| {
            let (row, column) = (index / 2, index % 2);
            let first = self.0[2 * row].mul(&other.0[column], format);
            let second = self.0[2 * row + 1].mul(&other.0[2 + column], format);
            FixedComplex {
                re: first.re + second.re,
                im: first.im + second.im,
            }
        }))
    }
    pub fn adjoint(&self) -> Self {
        Self(core::array::from_fn(|index| {
            let cell = &self.0[(index % 2) * 2 + index / 2];
            FixedComplex {
                re: cell.re.clone(),
                im: -&cell.im,
            }
        }))
    }
}

pub struct FibonacciLocal {
    format: FixedPointFormat,
    generators: [LocalMatrix; 3],
    associator: LocalMatrix,
}

/// Single-qubit corrections for the algebraic two-controlled-exchange CNOT
/// construction. Matrices are source-width fixed point; they still need
/// compilation into physical braids before carrier execution.
pub struct CnotLocalCorrections {
    pub target_basis: LocalMatrix,
    pub target_y: LocalMatrix,
    pub control_phase: FixedComplex,
}

pub fn cnot_local_corrections(source: &BigUint) -> Result<CnotLocalCorrections, String> {
    let algebra = FibonacciLocal::new(source)?;
    let format = algebra.format();
    let scale = format.scale().to_biguint().ok_or("invalid fixed scale")?;
    let scale_squared = &scale * &scale;
    let zero = FixedComplex { re: BigInt::zero(), im: BigInt::zero() };
    let unit = |n: i32, d: u32| FixedComplex::winding_twiddle(&BigInt::from(n), &BigUint::from(d), format);
    let v = LocalMatrix([unit(-3, 10)?, zero.clone(), zero.clone(), unit(3, 10)?]);
    let cos_gamma = unit(3, 10)?;
    let sin_squared = &scale_squared - (&cos_gamma.re * &cos_gamma.re).to_biguint().ok_or("invalid phase norm")?;
    let denominator = (BigUint::from(2u8) * sin_squared).sqrt();
    if denominator.is_zero() { return Err("degenerate CNOT exchange phase".into()); }
    let c = BigInt::from(&scale_squared / &denominator);
    let s = BigInt::from((&scale_squared - (&c * &c).to_biguint().ok_or("invalid rotation norm")?).sqrt());
    let y = LocalMatrix([
        FixedComplex { re: c.clone(), im: BigInt::zero() },
        FixedComplex { re: -&s, im: BigInt::zero() },
        FixedComplex { re: s.clone(), im: BigInt::zero() },
        FixedComplex { re: c, im: BigInt::zero() },
    ]);
    let w = v.multiply(&y, format).multiply(&v, format).multiply(&y.adjoint(), format);
    let k = LocalMatrix(core::array::from_fn(|i| FixedComplex {
        re: -&w.0[i].im,
        im: w.0[i].re.clone(),
    }));
    let z = &k.0[0].re;
    let scale_i = BigInt::from(scale.clone());
    let u_squared = ((BigInt::from(scale_squared.clone()) + &scale_i * z) >> 1usize)
        .to_biguint().ok_or("invalid CNOT eigenbasis norm")?;
    // The positive eigenspace formula is regular for this seed (u != 0).
    let u = BigInt::from(u_squared.sqrt());
    if u.is_zero() { return Err("CNOT correction eigenbasis is singular".into()); }
    let v_re = &k.0[1].re * &scale_i / (&u * 2u8);
    let v_im = -&k.0[1].im * &scale_i / (&u * 2u8);
    let a = LocalMatrix([
        FixedComplex { re: u.clone(), im: BigInt::zero() },
        FixedComplex { re: -&v_re, im: v_im.clone() },
        FixedComplex { re: v_re, im: v_im },
        FixedComplex { re: u, im: BigInt::zero() },
    ]);
    let h_component = BigInt::from((&scale_squared >> 1usize).sqrt());
    let h = LocalMatrix([
        FixedComplex { re: h_component.clone(), im: BigInt::zero() },
        FixedComplex { re: h_component.clone(), im: BigInt::zero() },
        FixedComplex { re: h_component.clone(), im: BigInt::zero() },
        FixedComplex { re: -h_component, im: BigInt::zero() },
    ]);
    let target_basis = a.multiply(&h, format);
    Ok(CnotLocalCorrections {
        target_basis,
        target_y: y,
        control_phase: unit(1, 20)?,
    })
}

impl FibonacciLocal {
    pub fn new(source: &BigUint) -> Result<Self, String> {
        if source.bits() < 128 {
            return Err("anyon factorization source must be at least 128 bits".into());
        }
        let format = FixedPointFormat::for_modulus(source)?;
        Self::with_format(format)
    }
    fn with_format(format: FixedPointFormat) -> Result<Self, String> {
        let scale = format.scale().to_biguint().ok_or("invalid fixed scale")?;
        let phi = (&scale + (BigUint::from(5u8) * &scale * &scale).sqrt()) >> 1usize;
        let inverse_phi = (&scale * &scale) / phi;
        let off_diagonal = (&inverse_phi * &scale).sqrt();
        let real = |value: BigInt| FixedComplex {
            re: value,
            im: BigInt::zero(),
        };
        let f = LocalMatrix([
            real(BigInt::from(inverse_phi.clone())),
            real(BigInt::from(off_diagonal.clone())),
            real(BigInt::from(off_diagonal)),
            real(-BigInt::from(inverse_phi)),
        ]);
        // Keep the chirality used by the kernel's existing r_symbol:
        // vacuum = 2/5 winding, tau = 7/10 winding.
        let r0 = FixedComplex::winding_twiddle(&BigInt::from(2u8), &BigUint::from(5u8), &format)?;
        let r1 = FixedComplex::winding_twiddle(&BigInt::from(7u8), &BigUint::from(10u8), &format)?;
        let zero = real(BigInt::zero());
        let diagonal = LocalMatrix([r0, zero.clone(), zero, r1]);
        let middle = f.multiply(&diagonal, &format).multiply(&f, &format);
        Ok(Self {
            format,
            generators: [diagonal.clone(), middle, diagonal],
            associator: f,
        })
    }
    pub fn format(&self) -> &FixedPointFormat {
        &self.format
    }
    pub(crate) fn associator(&self) -> &LocalMatrix {
        &self.associator
    }
    /// Fixed-precision local target for the streamed carrier's gate request.
    /// CNOT needs a two-qubit computational subspace and leakage checks.
    pub fn target(&self, gate: &BraidTarget) -> Result<LocalMatrix, String> {
        let zero = FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        };
        let one = FixedComplex {
            re: self.format.scale(),
            im: BigInt::zero(),
        };
        match gate {
            BraidTarget::X(_) => Ok(LocalMatrix([zero.clone(), one.clone(), one, zero])),
            BraidTarget::H(_) => {
                let scale = self
                    .format
                    .scale()
                    .to_biguint()
                    .ok_or("invalid fixed scale")?;
                let component = BigInt::from(((&scale * &scale) >> 1usize).sqrt());
                let positive = FixedComplex {
                    re: component.clone(),
                    im: BigInt::zero(),
                };
                let negative = FixedComplex {
                    re: -component,
                    im: BigInt::zero(),
                };
                Ok(LocalMatrix([
                    positive.clone(),
                    positive.clone(),
                    positive,
                    negative,
                ]))
            }
            BraidTarget::T { inverse, .. } => {
                let numerator = BigInt::from(if *inverse { -1i8 } else { 1i8 });
                let phase =
                    FixedComplex::winding_twiddle(&numerator, &BigUint::from(8u8), &self.format)?;
                Ok(LocalMatrix([one, zero.clone(), zero, phase]))
            }
            BraidTarget::Feedback {
                numerator,
                denominator_bits,
                ..
            } => {
                let source_width = usize::try_from(
                    self.format
                        .modulus_bits
                        .checked_add(1)
                        .ok_or("feedback source width overflow")?,
                )
                    .map_err(|_| "feedback source width exceeds host indexing")?;
                let precision_limit = source_width
                    .checked_mul(2)
                    .and_then(|width| width.checked_add(8))
                    .ok_or("feedback precision limit overflow")?;
                if *denominator_bits > precision_limit {
                    return Err("feedback denominator exceeds the source-bound phase precision".into());
                }
                let denominator = BigUint::one() << *denominator_bits;
                let phase = FixedComplex::winding_twiddle(
                    &(-BigInt::from(numerator.clone())),
                    &denominator,
                    &self.format,
                )?;
                Ok(LocalMatrix([one, zero.clone(), zero, phase]))
            }
            BraidTarget::Cnot { .. } => {
                Err("CNOT requires calibrated two-qubit braid lowering".into())
            }
        }
    }
    pub fn evaluate(&self, word: &[i32]) -> Result<LocalMatrix, String> {
        // Carry guard precision through the full word, then round once to the
        // source scale. This avoids accumulated unitary drift for long braids.
        // This guard policy is numerical; it is not a certified error bound.
        let length_bits = usize::BITS - word.len().max(1).leading_zeros();
        let guard = u64::from(length_bits) + 16;
        let w_bits = self
            .format
            .w_bits
            .checked_add(guard)
            .ok_or("braid precision overflow")?;
        usize::try_from(w_bits).map_err(|_| "braid precision exceeds host indexing")?;
        let guarded = Self::with_format(FixedPointFormat {
            modulus_bits: self.format.modulus_bits,
            w_bits,
        })?;
        let product = guarded.evaluate_inner(word)?;
        Ok(LocalMatrix(core::array::from_fn(|i| FixedComplex {
            re: &product.0[i].re >> guard as usize,
            im: &product.0[i].im >> guard as usize,
        })))
    }
    fn evaluate_inner(&self, word: &[i32]) -> Result<LocalMatrix, String> {
        let mut product = LocalMatrix::identity(&self.format);
        for &generator in word {
            let magnitude = generator.unsigned_abs();
            if !(1..=3).contains(&magnitude) {
                return Err("local braid generator outside 1..=3".into());
            }
            let gate = &self.generators[(magnitude - 1) as usize];
            product = if generator < 0 {
                gate.adjoint().multiply(&product, &self.format)
            } else {
                gate.multiply(&product, &self.format)
            };
        }
        Ok(product)
    }

    /// Reichardt's diagonal refinement, Eq. (5) of Carnahan, Zeuch and
    /// Bonesteel, Phys. Rev. A 93, 052328 (2016).
    /// https://web2.physics.fsu.edu/~bonesteel/papers/pra16_2.pdf
    /// U' = U R U† R³ U R³ U† R U. The project uses the conjugate
    /// R chirality of that paper; conjugation preserves the contraction.
    /// This emits actual braid generators, never an ideal diagonal matrix.
    /// Diagonal phases are not freely selectable by this refinement.
    /// No assertion about the moving-strand weave topology is made here.
    pub fn refine_diagonal_braid(
        &self,
        seed: &[i32],
        iterations: usize,
    ) -> Result<Vec<i32>, String> {
        self.evaluate(seed)?;
        let mut word = seed.to_vec();
        for _ in 0..iterations {
            let length = word
                .len()
                .checked_mul(5)
                .and_then(|n| n.checked_add(8))
                .ok_or("diagonal braid length overflow")?;
            let mut next = Vec::new();
            next.try_reserve_exact(length)
                .map_err(|_| "diagonal braid allocation failed")?;
            let inverse = |out: &mut Vec<i32>| {
                out.extend(word.iter().rev().map(|&g| -g));
            };
            // The matrix product is palindromic, so this chronological gate
            // order also gives the stated product under left multiplication.
            next.extend_from_slice(&word);
            next.push(1);
            inverse(&mut next);
            next.extend_from_slice(&[1, 1, 1]);
            next.extend_from_slice(&word);
            next.extend_from_slice(&[1, 1, 1]);
            inverse(&mut next);
            next.push(1);
            next.extend_from_slice(&word);
            debug_assert_eq!(next.len(), length);
            word = next;
        }
        Ok(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Signed;

    #[test]
    fn anyon_cnot_local_decomposition_128_and_192_bit_semiprimes() {
        for source in [
            "296650821743515430283258444261036507151",
            "3448910520600450090963625683018141073856509123385137086401",
        ] {
            let n = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert!(n.bits() >= 128);
            let corrections = cnot_local_corrections(&n).unwrap();
            let algebra = FibonacciLocal::new(&n).unwrap();
            let format = algebra.format();
            let v = LocalMatrix([
                FixedComplex::winding_twiddle(&BigInt::from(-3), &BigUint::from(10u8), format).unwrap(),
                FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                FixedComplex { re: BigInt::zero(), im: BigInt::zero() },
                FixedComplex::winding_twiddle(&BigInt::from(3), &BigUint::from(10u8), format).unwrap(),
            ]);
            let w = v.multiply(&corrections.target_y, format)
                .multiply(&v, format).multiply(&corrections.target_y.adjoint(), format);
            let i_w = LocalMatrix(core::array::from_fn(|i| FixedComplex {
                re: -&w.0[i].im, im: w.0[i].re.clone(),
            }));
            let identity = LocalMatrix::identity(format);
            let target_x = algebra.target(&BraidTarget::X(0)).unwrap();
            let observed = corrections.target_basis.adjoint().multiply(&i_w, format)
                .multiply(&corrections.target_basis, format);
            close(&observed, &target_x);
            close(&corrections.target_basis.adjoint().multiply(&corrections.target_basis, format), &identity);
            let expected_phase = FixedComplex::winding_twiddle(&BigInt::from(1), &BigUint::from(20u8), format).unwrap();
            assert_eq!(corrections.control_phase, expected_phase);
        }
    }

    #[test]
    fn anyon_diagonal_refinement_128_and_192_bit_semiprimes() {
        for source in [
            "296650821743515430283258444261036507151",
            "3448910520600450090963625683018141073856509123385137086401",
        ] {
            let n = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert!(n.bits() >= 128);
            let algebra = FibonacciLocal::new(&n).unwrap();
            let scale = algebra.format().scale();
            let scale_squared = &scale * &scale;
            let mass = |matrix: &LocalMatrix| {
                &matrix.0[1].re * &matrix.0[1].re + &matrix.0[1].im * &matrix.0[1].im
            };
            let mut previous = mass(&algebra.evaluate(&[2, 2, 2]).unwrap());
            for iteration in 1..=4 {
                let word = algebra
                    .refine_diagonal_braid(&[2, 2, 2], iteration)
                    .unwrap();
                let matrix = algebra.evaluate(&word).unwrap();
                let observed = mass(&matrix);
                let predicted = previous.pow(5u32) / scale_squared.pow(4u32);
                assert!((&observed - predicted).abs() < &scale * BigInt::from(4096u16));
                assert!(observed < previous);
                previous = observed;
                if iteration == 4 {
                    assert_eq!(word.len(), 3123);
                    assert!(matrix.0[1].re.abs() < BigInt::from(4096u16));
                    assert!(matrix.0[1].im.abs() < BigInt::from(4096u16));
                    close(
                        &matrix.adjoint().multiply(&matrix, algebra.format()),
                        &LocalMatrix::identity(algebra.format()),
                    );
                }
            }
            assert!(algebra.refine_diagonal_braid(&[i32::MIN], 1).is_err());
        }
    }

    fn close(first: &LocalMatrix, second: &LocalMatrix) {
        // Residual is measured in integer units of the source-derived scale.
        // This checks local numerical algebra, not a certified global bound.
        let tolerance = BigInt::from(1024u16);
        for (a, b) in first.0.iter().zip(&second.0) {
            assert!(
                (&a.re - &b.re).abs() <= tolerance,
                "real residual {}",
                (&a.re - &b.re).abs()
            );
            assert!(
                (&a.im - &b.im).abs() <= tolerance,
                "imaginary residual {}",
                (&a.im - &b.im).abs()
            );
        }
    }

    #[test]
    fn anyon_local_relations_for_128_and_192_bit_semiprimes() {
        for source in [
            "296650821743515430283258444261036507151",
            "3448910520600450090963625683018141073856509123385137086401",
        ] {
            let n = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert!(n.bits() >= 128);
            let algebra = FibonacciLocal::new(&n).unwrap();
            assert!(algebra.format().w_bits > 256);
            let identity = LocalMatrix::identity(algebra.format());
            for target in [BraidTarget::H(0), BraidTarget::X(0)] {
                let matrix = algebra.target(&target).unwrap();
                close(&matrix.multiply(&matrix, algebra.format()), &identity);
            }
            let t = algebra
                .target(&BraidTarget::T {
                    qubit: 0,
                    inverse: false,
                })
                .unwrap();
            let inverse_t = algebra
                .target(&BraidTarget::T {
                    qubit: 0,
                    inverse: true,
                })
                .unwrap();
            close(&t.multiply(&inverse_t, algebra.format()), &identity);
            let feedback = algebra
                .target(&BraidTarget::Feedback {
                    qubit: 0,
                    numerator: BigUint::one(),
                    denominator_bits: 2 * n.bits() as usize,
                })
                .unwrap();
            assert!(feedback.0[3].im.is_negative());
            assert!(algebra
                .target(&BraidTarget::Cnot {
                    control: 0,
                    target: 1
                })
                .is_err());
            for generator in 1..=3 {
                close(
                    &algebra.evaluate(&[generator, -generator]).unwrap(),
                    &identity,
                );
            }
            close(
                &algebra.evaluate(&[1, 2, 1]).unwrap(),
                &algebra.evaluate(&[2, 1, 2]).unwrap(),
            );
            close(
                &algebra.evaluate(&[2, 3, 2]).unwrap(),
                &algebra.evaluate(&[3, 2, 3]).unwrap(),
            );
            assert!(algebra.evaluate(&[0]).is_err());
            assert!(algebra.evaluate(&[4]).is_err());
            assert!(algebra.evaluate(&[i32::MIN]).is_err());
            // Winding coordinates can grow without overflowing or changing
            // the represented phase after adding an integral number of turns.
            let denominator = BigUint::from(5u8);
            let huge = BigInt::from(&denominator * &n) + BigInt::from(2u8);
            let a = FixedComplex::winding_twiddle(&huge, &denominator, algebra.format()).unwrap();
            let b =
                FixedComplex::winding_twiddle(&BigInt::from(2u8), &denominator, algebra.format())
                    .unwrap();
            assert_eq!(a, b);
        }
    }
}
