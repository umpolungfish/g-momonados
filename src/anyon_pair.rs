//! Local two-qubit Fibonacci fusion algebra, including its leakage channel.
//! Five fusion channels are fixed by gate arity, not factorization width.
use crate::anyon_local::FibonacciLocal;
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, Zero};

// Six tau anyons with vacuum total charge, in canonical running-total order.
const CHANNELS: [[usize; 6]; 5] = [
    [1, 0, 1, 0, 1, 0],
    [1, 0, 1, 1, 1, 0],
    [1, 1, 0, 1, 1, 0],
    [1, 1, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 0],
];
// Little-endian logical bits: first pair channel m_2, last pair channel m_4.
// Each triple has tau charge. Channel 2 has vacuum first-triple charge and leaks.
pub const COMPUTATIONAL_CHANNELS: [usize; 4] = [0, 3, 1, 4];
pub const LEAKAGE_CHANNEL: usize = 2;

#[derive(Clone, Debug)]
pub struct PairMatrix(pub [FixedComplex; 25]);

/// Rounded local diagnostics in integer units of the fixed-point scale.
/// These are synthesis diagnostics, not a certified accumulated circuit bound.
#[derive(Clone, Debug)]
pub struct CnotResidual {
    pub computational: BigUint,
    pub leakage: BigUint,
    pub unitarity: BigUint,
}

#[derive(Clone, Debug)]
pub struct CnotBraid {
    pub word: Vec<i32>,
    pub residual: CnotResidual,
}

#[derive(Debug)]
pub enum CnotCompileError {
    Configuration(&'static str),
    Budget { best: CnotBraid, depth: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositeLayout {
    /// Original object identifiers, in their final left-to-right order.
    pub order: [usize; 3],
    pub widths: [usize; 3],
}

impl CnotResidual {
    pub fn maximum(&self) -> BigUint {
        self.computational
            .clone()
            .max(self.leakage.clone())
            .max(self.unitarity.clone())
    }
}
impl PairMatrix {
    pub fn identity(format: &FixedPointFormat) -> Self {
        Self(core::array::from_fn(|i| FixedComplex {
            re: if i / 5 == i % 5 {
                format.scale()
            } else {
                BigInt::zero()
            },
            im: BigInt::zero(),
        }))
    }
    pub fn multiply(&self, other: &Self, format: &FixedPointFormat) -> Self {
        Self(core::array::from_fn(|i| {
            let mut result = FixedComplex {
                re: BigInt::zero(),
                im: BigInt::zero(),
            };
            for k in 0..5 {
                let left = &self.0[5 * (i / 5) + k];
                let right = &other.0[5 * k + i % 5];
                if (left.re.is_zero() && left.im.is_zero())
                    || (right.re.is_zero() && right.im.is_zero())
                {
                    continue;
                }
                let product = left.mul(right, format);
                result.re += product.re;
                result.im += product.im;
            }
            result
        }))
    }
    pub fn adjoint(&self) -> Self {
        Self(core::array::from_fn(|i| {
            let cell = &self.0[(i % 5) * 5 + i / 5];
            FixedComplex {
                re: cell.re.clone(),
                im: -&cell.im,
            }
        }))
    }
    /// Maximum leakage amplitude component for any computational input.
    /// This is a rounded local numerical residual, not a global error proof.
    pub fn leakage(&self) -> BigUint {
        COMPUTATIONAL_CHANNELS
            .iter()
            .fold(BigUint::zero(), |maximum, &column| {
                let cell = &self.0[5 * LEAKAGE_CHANNEL + column];
                maximum
                    .max(cell.re.abs().to_biguint().unwrap())
                    .max(cell.im.abs().to_biguint().unwrap())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &PairMatrix, b: &PairMatrix) {
        let tolerance = BigInt::from(4096u16);
        for (a, b) in a.0.iter().zip(&b.0) {
            assert!((&a.re - &b.re).abs() <= tolerance);
            assert!((&a.im - &b.im).abs() <= tolerance);
        }
    }

    #[test]
    fn anyon_pair_relations_for_128_and_192_bit_semiprimes() {
        for source in [
            "296650821743515430283258444261036507151",
            "3448910520600450090963625683018141073856509123385137086401",
        ] {
            let n = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert!(n.bits() >= 128);
            let algebra = FibonacciPair::new(&n).unwrap();
            let identity = PairMatrix::identity(algebra.format());
            for generator in 1..=5 {
                close(
                    &algebra.evaluate(&[generator, -generator]).unwrap(),
                    &identity,
                );
                if generator < 5 {
                    close(
                        &algebra
                            .evaluate(&[generator, generator + 1, generator])
                            .unwrap(),
                        &algebra
                            .evaluate(&[generator + 1, generator, generator + 1])
                            .unwrap(),
                    );
                }
                for distant in generator + 2..=5 {
                    close(
                        &algebra.evaluate(&[generator, distant]).unwrap(),
                        &algebra.evaluate(&[distant, generator]).unwrap(),
                    );
                }
            }
            // Braids within either triple preserve its tau total charge;
            // the middle exchange must retain its explicit leakage channel.
            for generator in [1, 2, 4, 5] {
                assert!(
                    algebra.evaluate(&[generator]).unwrap().leakage() <= BigUint::from(4096u16)
                );
            }
            assert!(algebra.evaluate(&[3]).unwrap().leakage() > BigUint::from(4096u16));
            let cnot = algebra.cnot_target();
            close(&cnot.multiply(&cnot, algebra.format()), &identity);
            assert!(cnot.leakage().is_zero());
            let residual = algebra.cnot_residual(&cnot);
            assert!(residual.computational.is_zero());
            assert!(residual.leakage.is_zero());
            assert!(residual.unitarity.is_zero());
            // Global sign and an independent leakage-channel sign are allowed.
            let mut phased = cnot.clone();
            for entry in &mut phased.0 {
                entry.re = -&entry.re;
                entry.im = -&entry.im;
            }
            phased.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = algebra.format().scale();
            let residual = algebra.cnot_residual(&phased);
            assert!(residual.computational.is_zero());
            assert!(residual.leakage.is_zero());
            assert!(residual.unitarity.is_zero());
            let mut projected = cnot.clone();
            projected.0[0].re = BigInt::zero();
            let residual = algebra.cnot_residual(&projected);
            assert!(!residual.computational.is_zero());
            assert!(!residual.unitarity.is_zero());
            let residual = algebra.cnot_residual(&algebra.evaluate(&[3]).unwrap());
            assert!(!residual.leakage.is_zero());
            for invalid in [0, 6, i32::MIN] {
                assert!(algebra.evaluate(&[invalid]).is_err());
            }
        }
    }

    #[test]
    fn anyon_cnot_compile_budget_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let algebra = FibonacciPair::new(&n).unwrap();
        let tolerance = BigUint::from(4096u16);
        assert!(matches!(
            algebra.compile_cnot(2, 0, &tolerance),
            Err(CnotCompileError::Configuration(_))
        ));
        assert!(matches!(
            algebra.compile_cnot(2, 8, &algebra.format().scale().to_biguint().unwrap()),
            Err(CnotCompileError::Configuration(_))
        ));
        match algebra.compile_cnot(2, 8, &tolerance) {
            Err(CnotCompileError::Budget { best, depth }) => {
                assert_eq!(depth, 2);
                assert!(best.word.len() <= depth);
                let checked = algebra.cnot_residual(&algebra.evaluate(&best.word).unwrap());
                assert_eq!(checked.maximum(), best.residual.maximum());
                assert!(checked.maximum() > tolerance);
            }
            other => panic!("short CNOT search must report its unmet precision: {other:?}"),
        }
    }

    #[test]
    fn anyon_composite_expansion_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let algebra = FibonacciPair::new(&n).unwrap();
        let mut word = Vec::new();
        let layout = algebra
            .emit_composite_braid(&[1, 2, -1], [2, 1, 1], 1, |g| {
                word.push(g);
                Ok(())
            })
            .unwrap();
        assert_eq!(word, [3, 2, 4, 3, -2]);
        assert_eq!(layout.order, [2, 1, 0]);
        assert_eq!(layout.widths, [1, 1, 2]);
        let mut inverse = Vec::new();
        let restored = algebra
            .emit_composite_braid(&[1, -2, -1], layout.widths, 1, |g| {
                inverse.push(g);
                Ok(())
            })
            .unwrap();
        assert_eq!(inverse, word.iter().rev().map(|&g| -g).collect::<Vec<_>>());
        assert_eq!(restored.widths, [2, 1, 1]);
        word.extend(inverse);
        close(
            &algebra.evaluate(&word).unwrap(),
            &PairMatrix::identity(algebra.format()),
        );
        let mut calls = 0;
        assert!(algebra
            .emit_composite_braid(&[1, 0], [2, 1, 1], 1, |_| {
                calls += 1;
                Ok(())
            })
            .is_err());
        assert_eq!(calls, 0);
        assert!(algebra
            .emit_composite_braid(&[], [usize::MAX, 1, 1], 1, |_| Ok(()))
            .is_err());
    }

    #[test]
    fn anyon_controlled_exchange_leakage_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let algebra = FibonacciPair::new(&n).unwrap();
        let mut previous = algebra.format().scale().to_biguint().unwrap();
        for iteration in 0..=2 {
            let word = algebra.controlled_double_exchange(iteration).unwrap();
            let matrix = algebra.evaluate(&word).unwrap();
            let leakage = matrix.leakage();
            std::println!(
                "iteration={iteration} length={} leakage={leakage} scale={}",
                word.len(),
                algebra.format().scale()
            );
            assert!(leakage < previous);
            previous = leakage;
            if iteration == 2 {
                assert!(previous < algebra.format().scale().to_biguint().unwrap() >> 35usize);
                let channels = algebra.in_pair_channels(&matrix);
                let local = FibonacciLocal::new(&n).unwrap();
                let r = local.evaluate(&[1, 1]).unwrap();
                let tolerance = algebra.format().scale() >> 35usize;
                for input in 0usize..4 {
                    for output in 0usize..4 {
                        let expected = if input != output {
                            FixedComplex {
                                re: BigInt::zero(),
                                im: BigInt::zero(),
                            }
                        } else if input & 1 == 0 {
                            FixedComplex {
                                re: algebra.format().scale(),
                                im: BigInt::zero(),
                            }
                        } else {
                            r.0[3 * (input >> 1)].clone()
                        };
                        let actual = &channels.0
                            [5 * COMPUTATIONAL_CHANNELS[output] + COMPUTATIONAL_CHANNELS[input]];
                        assert!(
                            (&actual.re - &expected.re).abs() < tolerance,
                            "controlled exchange real residual input={input} output={output}: {}",
                            (&actual.re - &expected.re).abs()
                        );
                        assert!((&actual.im - &expected.im).abs() < tolerance, "controlled exchange imaginary residual input={input} output={output}: {}", (&actual.im - &expected.im).abs());
                    }
                }
                // Apply the computational block to a product |+,+> input.
                // The common normalization is omitted. A nonzero 2x2
                // coefficient determinant witnesses entanglement.
                let output: [FixedComplex; 4] = core::array::from_fn(|row| {
                    let mut value = FixedComplex {
                        re: BigInt::zero(),
                        im: BigInt::zero(),
                    };
                    for &column in &COMPUTATIONAL_CHANNELS {
                        let cell = &matrix.0[5 * COMPUTATIONAL_CHANNELS[row] + column];
                        value.re += &cell.re;
                        value.im += &cell.im;
                    }
                    value
                });
                let diagonal = output[0].mul(&output[3], algebra.format());
                let off_diagonal = output[1].mul(&output[2], algebra.format());
                let witness = (&diagonal.re - &off_diagonal.re)
                    .abs()
                    .max((&diagonal.im - &off_diagonal.im).abs());
                assert!(witness > algebra.format().scale() >> 8usize);
                let cnot = algebra.cnot_operator_from_exchange(iteration).unwrap();
                let residual = algebra.cnot_residual(&cnot);
                assert!(residual.maximum() < algebra.format().scale().to_biguint().unwrap() >> 35usize,
                    "CNOT comp={}/scale leakage={}/scale unitary={}/scale",
                    residual.computational, residual.leakage, residual.unitarity);
            }
        }
    }
}

pub struct FibonacciPair {
    source: BigUint,
    format: FixedPointFormat,
    generators: [PairMatrix; 5],
    pair_channels: PairMatrix,
}
impl FibonacciPair {
    /// Assemble the controlled-exchange matrix with its derived one-qubit
    /// corrections, then express it as a CNOT operator. The correction matrices
    /// are ideal targets; this does not yet compile them to physical braids.
    pub fn cnot_operator_from_exchange(&self, iterations: usize) -> Result<PairMatrix, String> {
        use crate::anyon_local::{cnot_local_corrections, LocalMatrix};
        let physical = self.evaluate(&self.controlled_double_exchange(iterations)?)?;
        let controlled_exchange = self.in_pair_channels(&physical);
        let corrections = cnot_local_corrections(&self.source)?;
        let local_pair = |gate: &LocalMatrix, control: bool| {
            let mut matrix = PairMatrix(core::array::from_fn(|_| FixedComplex { re: BigInt::zero(), im: BigInt::zero() }));
            matrix.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = self.format.scale();
            for input in 0usize..4 {
                for output in 0usize..4 {
                    if (input & 1 != output & 1) || (control && input & 1 == 0) {
                        continue;
                    }
                    let row = output >> 1;
                    let column = input >> 1;
                    matrix.0[5 * COMPUTATIONAL_CHANNELS[output] + COMPUTATIONAL_CHANNELS[input]] = gate.0[2 * row + column].clone();
                }
            }
            matrix
        };
        let control_phase = |phase: &FixedComplex| {
            let mut matrix = PairMatrix::identity(&self.format);
            for bit in [1usize, 3] {
                let channel = COMPUTATIONAL_CHANNELS[bit];
                matrix.0[5 * channel + channel] = phase.clone();
            }
            matrix
        };
        let basis = local_pair(&corrections.target_basis, false);
        let basis_inverse = basis.adjoint();
        let y = local_pair(&corrections.target_y, false);
        let y_inverse = y.adjoint();
        let control = control_phase(&corrections.control_phase);
        Ok(control.multiply(&basis_inverse, &self.format)
            .multiply(&controlled_exchange, &self.format)
            .multiply(&y, &self.format)
            .multiply(&controlled_exchange, &self.format)
            .multiply(&y_inverse, &self.format)
            .multiply(&basis, &self.format))
    }

    /// Construct the controlled-double-exchange braid of Fig. 5 in
    /// Carnahan, Zeuch and Bonesteel (2016). The control pair occupies strands
    /// 2,3 and the target pair 4,5. Their pair-channel logical basis differs
    /// from COMPUTATIONAL_CHANNELS; local recoupling is needed for CNOT use.
    /// This returns a physical word whose leakage must be measured, not a
    /// calibrated CNOT or an ideal controlled gate matrix.
    pub fn controlled_double_exchange(&self, iterations: usize) -> Result<Vec<i32>, String> {
        use crate::anyon_weave::{compile_exchange, refine_sequence, FusionStep};
        let sequence = refine_sequence(
            &[FusionStep::F, FusionStep::R(3), FusionStep::F],
            iterations,
        )?;
        let injection = compile_exchange(&sequence)?;
        let mut word = Vec::new();
        let injected = self.emit_composite_braid(&injection.word, [2, 1, 1], 1, |g| {
            word.push(g);
            Ok(())
        })?;
        if injected.order[1] != 0 {
            return Err("exchange weave did not inject the control pair".into());
        }
        let after_exchange = self.emit_composite_braid(&[2, 2], injected.widths, 1, |g| {
            word.push(g);
            Ok(())
        })?;
        let inverse: Vec<_> = injection.word.iter().rev().map(|&g| -g).collect();
        let restored = self.emit_composite_braid(&inverse, after_exchange.widths, 1, |g| {
            word.push(g);
            Ok(())
        })?;
        if restored.widths != [2, 1, 1] {
            return Err("exchange weave failed to restore the control pair".into());
        }
        Ok(word)
    }

    /// Lift crossings of three composite objects to elementary six-anyon
    /// crossings. Each object keeps its internal strand order. This is a
    /// topological expansion; it does not identify an arbitrary lifted word
    /// as a controlled gate or discard its relative phases.
    /// A callback error can leave a partial execution; its caller must abort.
    pub fn emit_composite_braid<F>(
        &self,
        word: &[i32],
        widths: [usize; 3],
        offset: usize,
        mut emit: F,
    ) -> Result<CompositeLayout, String>
    where
        F: FnMut(i32) -> Result<(), String>,
    {
        if widths.contains(&0) {
            return Err("composite braid objects must contain strands".into());
        }
        let end = widths
            .iter()
            .try_fold(offset, |total, width| total.checked_add(*width))
            .ok_or("composite braid layout overflow")?;
        if end > 6 {
            return Err("composite braid lies outside six-anyon space".into());
        }
        if word.iter().any(|g| !(1..=2).contains(&g.unsigned_abs())) {
            return Err("composite braid generator outside 1..=2".into());
        }
        let mut layout = CompositeLayout {
            order: [0, 1, 2],
            widths,
        };
        for &generator in word {
            let pair = generator.unsigned_abs() as usize - 1;
            let start = offset + layout.widths[..pair].iter().sum::<usize>();
            let left = layout.widths[pair];
            let right = layout.widths[pair + 1];
            // Move the right object's strands through the left object, one
            // strand at a time. Every cross-object pair crosses exactly once.
            for strand in 0..right {
                for index in (start + strand + 1..=start + strand + left).rev() {
                    let crossing = index as i32;
                    emit(if generator < 0 { -crossing } else { crossing })?;
                }
            }
            layout.widths.swap(pair, pair + 1);
            layout.order.swap(pair, pair + 1);
        }
        Ok(layout)
    }

    pub fn new(source: &BigUint) -> Result<Self, String> {
        let local = FibonacciLocal::new(source)?;
        let format = local.format().clone();
        let r = local.evaluate(&[1])?;
        let f = local.associator();
        let coefficient = |left: usize, new: usize, old: usize, right: usize| -> FixedComplex {
            if left == 0 {
                FixedComplex {
                    re: if old == 1 && new == right {
                        format.scale()
                    } else {
                        BigInt::zero()
                    },
                    im: BigInt::zero(),
                }
            } else if right == 1 {
                f.0[2 * new + old].clone()
            } else {
                FixedComplex {
                    re: if old == 1 && new == 1 {
                        format.scale()
                    } else {
                        BigInt::zero()
                    },
                    im: BigInt::zero(),
                }
            }
        };
        let generators = core::array::from_fn(|index| {
            let k = index + 1;
            PairMatrix(core::array::from_fn(|entry| {
                let (row, column) = (entry / 5, entry % 5);
                if k == 1 {
                    return if row == column {
                        r.0[3 * CHANNELS[column][1]].clone()
                    } else {
                        FixedComplex {
                            re: BigInt::zero(),
                            im: BigInt::zero(),
                        }
                    };
                }
                let varying = k - 1;
                if (0..6).any(|i| i != varying && CHANNELS[row][i] != CHANNELS[column][i]) {
                    return FixedComplex {
                        re: BigInt::zero(),
                        im: BigInt::zero(),
                    };
                }
                let left = CHANNELS[column][k - 2];
                let right = if k < 5 { CHANNELS[column][k] } else { 0 };
                let mut result = FixedComplex {
                    re: BigInt::zero(),
                    im: BigInt::zero(),
                };
                for channel in 0..2 {
                    let first = coefficient(left, channel, CHANNELS[row][varying], right);
                    let second = coefficient(left, channel, CHANNELS[column][varying], right);
                    let product = first.mul(&r.0[3 * channel], &format).mul(&second, &format);
                    result.re += product.re;
                    result.im += product.im;
                }
                result
            }))
        });
        // Change the first triple's channel from pair 1,2 to pair 2,3,
        // and the last triple's channel from pair 5,6 to pair 4,5.
        // This is F tensor F on the four computational channels, with the
        // noncomputational channel retained as a separate one-dimensional block.
        let pair_channels = PairMatrix(core::array::from_fn(|entry| {
            let (row, column) = (entry / 5, entry % 5);
            let row_bit = COMPUTATIONAL_CHANNELS.iter().position(|&i| i == row);
            let column_bit = COMPUTATIONAL_CHANNELS.iter().position(|&i| i == column);
            match (row_bit, column_bit) {
                (Some(a), Some(b)) => {
                    f.0[2 * (a & 1) + (b & 1)].mul(&f.0[2 * (a >> 1) + (b >> 1)], &format)
                }
                _ => FixedComplex {
                    re: if row == LEAKAGE_CHANNEL && column == LEAKAGE_CHANNEL {
                        format.scale()
                    } else {
                        BigInt::zero()
                    },
                    im: BigInt::zero(),
                },
            }
        }));
        Ok(Self {
            source: source.clone(),
            format,
            generators,
            pair_channels,
        })
    }
    pub fn format(&self) -> &FixedPointFormat {
        &self.format
    }
    /// Express a physical operator in the control-pair (2,3), target-pair
    /// (4,5) channel basis. This is a description change, not an emitted F gate.
    pub fn in_pair_channels(&self, physical: &PairMatrix) -> PairMatrix {
        self.pair_channels
            .adjoint()
            .multiply(physical, &self.format)
            .multiply(&self.pair_channels, &self.format)
    }
    pub fn evaluate(&self, word: &[i32]) -> Result<PairMatrix, String> {
        let gates: Vec<PairMatrix> = self.generators.iter()
            .flat_map(|gate| [gate.clone(), gate.adjoint()])
            .collect();
        let supports: Vec<Vec<Vec<(usize, FixedComplex)>>> = gates.iter().map(|gate| {
            (0..5).map(|row| {
                (0..5).filter_map(|column| {
                    let value = &gate.0[5 * row + column];
                    if value.re.is_zero() && value.im.is_zero() { None }
                    else { Some((column, value.clone())) }
                }).collect()
            }).collect()
        }).collect();
        let mut result = PairMatrix::identity(&self.format);
        for &generator in word {
            let magnitude = generator.unsigned_abs();
            if !(1..=5).contains(&magnitude) {
                return Err("two-qubit braid generator outside 1..=5".into());
            }
            let gate_index = 2 * (magnitude as usize - 1) + usize::from(generator < 0);
            result = PairMatrix(core::array::from_fn(|index| {
                let row = index / 5;
                let column = index % 5;
                let mut value = FixedComplex { re: BigInt::zero(), im: BigInt::zero() };
                for (inner, coefficient) in &supports[gate_index][row] {
                    let product = coefficient.mul(&result.0[5 * inner + column], &self.format);
                    value.re += product.re;
                    value.im += product.im;
                }
                value
            }));
        }
        Ok(result)
    }
    /// CNOT target extension: first logical bit controls the second, and the
    /// leakage channel is fixed. The target is not a synthesized braid.
    pub fn cnot_target(&self) -> PairMatrix {
        let mut result = PairMatrix(core::array::from_fn(|_| FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        }));
        result.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = self.format.scale();
        for input in 0usize..4 {
            let output = if input & 1 != 0 { input ^ 2 } else { input };
            result.0[5 * COMPUTATIONAL_CHANNELS[output] + COMPUTATIONAL_CHANNELS[input]].re =
                self.format.scale();
        }
        result
    }

    /// Compare the computational block to CNOT up to one common phase.
    /// The unused channel may have its own phase; computational inputs must
    /// still have negligible amplitude entering it. Unitarity is checked on
    /// all five channels, so a scaled-down or projected operator cannot pass.
    pub fn cnot_residual(&self, candidate: &PairMatrix) -> CnotResidual {
        // Align one common phase using the whole target overlap. A single
        // matrix entry can have an inaccurate phase or vanish during search.
        let mut overlap = FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        };
        for input in 0usize..4 {
            let output = if input & 1 != 0 { input ^ 2 } else { input };
            let cell =
                &candidate.0[5 * COMPUTATIONAL_CHANNELS[output] + COMPUTATIONAL_CHANNELS[input]];
            overlap.re += &cell.re;
            overlap.im += &cell.im;
        }
        let magnitude = (&overlap.re * &overlap.re + &overlap.im * &overlap.im)
            .to_biguint()
            .unwrap()
            .sqrt();
        let phase = if magnitude.is_zero() {
            FixedComplex {
                re: self.format.scale(),
                im: BigInt::zero(),
            }
        } else {
            let magnitude = BigInt::from(magnitude);
            FixedComplex {
                re: &overlap.re * self.format.scale() / &magnitude,
                im: &overlap.im * self.format.scale() / magnitude,
            }
        };
        let mut computational = BigUint::zero();
        for input in 0usize..4 {
            let output = if input & 1 != 0 { input ^ 2 } else { input };
            for row in 0usize..4 {
                let cell =
                    &candidate.0[5 * COMPUTATIONAL_CHANNELS[row] + COMPUTATIONAL_CHANNELS[input]];
                let (re, im) = if row == output {
                    (&cell.re - &phase.re, &cell.im - &phase.im)
                } else {
                    (cell.re.clone(), cell.im.clone())
                };
                computational = computational
                    .max(re.abs().to_biguint().unwrap())
                    .max(im.abs().to_biguint().unwrap());
            }
        }
        let norm = candidate.adjoint().multiply(candidate, &self.format);
        let identity = PairMatrix::identity(&self.format);
        let unitarity = norm
            .0
            .iter()
            .zip(&identity.0)
            .fold(BigUint::zero(), |maximum, (a, b)| {
                maximum
                    .max((&a.re - &b.re).abs().to_biguint().unwrap())
                    .max((&a.im - &b.im).abs().to_biguint().unwrap())
            });
        CnotResidual {
            computational,
            leakage: candidate.leakage(),
            unitarity,
        }
    }

    /// Search local braid words while retaining at most two bounded beams.
    /// Memory does not enumerate the factorization register's fusion states.
    /// This heuristic has no guarantee of finding a word within its budget.
    /// Acceptance uses rounded local residuals, not a global error certificate.
    pub fn compile_cnot(
        &self,
        max_depth: usize,
        beam_width: usize,
        tolerance: &BigUint,
    ) -> Result<CnotBraid, CnotCompileError> {
        if beam_width == 0 {
            return Err(CnotCompileError::Configuration(
                "CNOT beam width must be positive",
            ));
        }
        let scale = self.format.scale().to_biguint().unwrap();
        if tolerance >= &(&scale >> 2usize) {
            return Err(CnotCompileError::Configuration(
                "CNOT tolerance must be below one quarter of the scale",
            ));
        }
        #[derive(Clone)]
        struct Candidate {
            braid: CnotBraid,
            matrix: PairMatrix,
            score: BigUint,
        }
        let identity = PairMatrix::identity(&self.format);
        let residual = self.cnot_residual(&identity);
        let mut best = Candidate {
            score: residual.maximum(),
            braid: CnotBraid {
                word: Vec::new(),
                residual,
            },
            matrix: identity,
        };
        let mut beam = Vec::new();
        beam.try_reserve(beam_width)
            .map_err(|_| CnotCompileError::Configuration("CNOT beam allocation failed"))?;
        beam.push(best.clone());
        let generators: [(i32, PairMatrix); 10] = core::array::from_fn(|i| {
            let generator = if i < 5 { i as i32 + 1 } else { -(i as i32 - 4) };
            let matrix = if generator > 0 {
                self.generators[(generator - 1) as usize].clone()
            } else {
                self.generators[(-generator - 1) as usize].adjoint()
            };
            (generator, matrix)
        });
        for depth in 1..=max_depth {
            let mut next: Vec<Candidate> = Vec::new();
            next.try_reserve(beam_width)
                .map_err(|_| CnotCompileError::Configuration("CNOT beam allocation failed"))?;
            for parent in &beam {
                for (generator, gate) in &generators {
                    if parent.braid.word.last() == Some(&(-generator)) {
                        continue;
                    }
                    // Distant generators commute, including their inverses.
                    // Keep one sorted representative rather than fill the
                    // beam with cancelling commutators and their roundoff.
                    if let Some(&previous) = parent.braid.word.last() {
                        if previous.unsigned_abs().abs_diff(generator.unsigned_abs()) > 1
                            && previous > *generator
                        {
                            continue;
                        }
                    }
                    let matrix = gate.multiply(&parent.matrix, &self.format);
                    let residual = self.cnot_residual(&matrix);
                    let score = residual.maximum();
                    if score > *tolerance
                        && next.len() == beam_width
                        && score >= next.last().unwrap().score
                    {
                        continue;
                    }
                    let mut word = parent.braid.word.clone();
                    word.push(*generator);
                    let candidate = Candidate {
                        braid: CnotBraid { word, residual },
                        matrix,
                        score,
                    };
                    if candidate.score <= *tolerance {
                        // Re-evaluate the returned word independently of the
                        // retained incremental matrix before accepting it.
                        let checked = self.evaluate(&candidate.braid.word).map_err(|_| {
                            CnotCompileError::Configuration("invalid synthesized CNOT word")
                        })?;
                        let residual = self.cnot_residual(&checked);
                        if residual.maximum() <= *tolerance {
                            return Ok(CnotBraid {
                                word: candidate.braid.word,
                                residual,
                            });
                        }
                    }
                    if candidate.score < best.score {
                        best = candidate.clone();
                    }
                    let position = next.partition_point(|entry| entry.score <= candidate.score);
                    next.insert(position, candidate);
                    if next.len() > beam_width {
                        next.pop();
                    }
                }
            }
            beam = next;
            if beam.is_empty() {
                return Err(CnotCompileError::Budget {
                    best: best.braid,
                    depth,
                });
            }
        }
        Err(CnotCompileError::Budget {
            best: best.braid,
            depth: max_depth,
        })
    }
}
