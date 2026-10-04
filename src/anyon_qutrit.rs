//! Three computational fusion channels, with source-width fixed arithmetic.
//! Four tau anyons of total tau have three running-charge paths. Every path
//! below is computational; the third channel is retained by gates and readout.
use crate::anyon_fusion_kernel::{sample_born_masses, FusionKernel};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::string::String;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, Zero};

/// A phase numerator over 3^m. Digits are appended in least-significant order,
/// matching recycled inverse-Fourier measurement order.
pub struct QutritPhaseReadout {
    numerator: BigUint,
    denominator: BigUint,
    digits: usize,
}
impl Default for QutritPhaseReadout {
    fn default() -> Self {
        Self { numerator: BigUint::zero(), denominator: BigUint::one(), digits: 0 }
    }
}
impl QutritPhaseReadout {
    pub fn push(&mut self, value: Trit) -> Result<(), String> {
        let digits = self.digits.checked_add(1).ok_or("qutrit phase width overflow")?;
        self.numerator += &self.denominator * (value as u8);
        self.denominator *= 3u8;
        self.digits = digits;
        Ok(())
    }
    pub fn numerator(&self) -> &BigUint { &self.numerator }
    pub fn denominator(&self) -> &BigUint { &self.denominator }
    pub fn digits(&self) -> usize { self.digits }
    pub fn close(&self, evidence: &mut crate::phase_unbraid::PhaseReadoutAccumulator,
        source: &BigUint, base: &BigUint) -> Result<Option<(BigUint, BigUint, BigUint)>, String> {
        if self.digits == 0 || source.bits() < 128 { return Err("qutrit factor phase needs a readout and a source of at least 128 bits".into()); }
        evidence.close_fraction(&self.numerator, &self.denominator, base, source)
    }
}

pub const QUTRIT_PATHS: [[u8; 4]; 3] = [[1, 0, 1, 1], [1, 1, 0, 1], [1, 1, 1, 1]];

/// Numerical outcomes. These indices do not classify Belnap evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Trit {
    Zero = 0,
    One = 1,
    Two = 2,
}
impl TryFrom<u8> for Trit {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            _ => Err("qutrit outcome must be 0, 1, or 2".into()),
        }
    }
}

fn zero() -> FixedComplex {
    FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }
}

#[derive(Clone, Debug)]
pub struct QutritMatrix {
    cells: [FixedComplex; 9],
}
impl QutritMatrix {
    pub fn cells(&self) -> &[FixedComplex; 9] {
        &self.cells
    }
    pub fn identity(format: &FixedPointFormat) -> Self {
        Self {
            cells: core::array::from_fn(|i| {
                if i / 3 == i % 3 {
                    FixedComplex {
                        re: format.scale(),
                        im: BigInt::zero(),
                    }
                } else {
                    zero()
                }
            }),
        }
    }
    pub fn multiply(&self, rhs: &Self, format: &FixedPointFormat) -> Self {
        Self {
            cells: core::array::from_fn(|i| {
                let mut value = zero();
                for k in 0..3 {
                    let term = self.cells[3 * (i / 3) + k].mul(&rhs.cells[3 * k + i % 3], format);
                    value.re += term.re;
                    value.im += term.im;
                }
                value
            }),
        }
    }
    pub fn adjoint(&self) -> Self {
        Self {
            cells: core::array::from_fn(|i| {
                let value = &self.cells[3 * (i % 3) + i / 3];
                FixedComplex {
                    re: value.re.clone(),
                    im: -&value.im,
                }
            }),
        }
    }
    pub fn maximum_difference(&self, other: &Self) -> BigUint {
        self.cells
            .iter()
            .zip(&other.cells)
            .fold(BigUint::zero(), |error, (a, b)| {
                error
                    .max((&a.re - &b.re).abs().to_biguint().unwrap())
                    .max((&a.im - &b.im).abs().to_biguint().unwrap())
            })
    }
    pub fn unitarity_residual(&self, format: &FixedPointFormat) -> BigUint {
        self.adjoint()
            .multiply(self, format)
            .maximum_difference(&Self::identity(format))
    }
}

pub struct FibonacciQutrit {
    format: FixedPointFormat,
    generators: [QutritMatrix; 3],
}
impl FibonacciQutrit {
    pub fn new(source: &BigUint) -> Result<Self, String> {
        let kernel = FusionKernel::new(source)?;
        let mut generators: [QutritMatrix; 3] = core::array::from_fn(|_| QutritMatrix {
            cells: core::array::from_fn(|_| zero()),
        });
        for (index, generator) in generators.iter_mut().enumerate() {
            for (column, path) in QUTRIT_PATHS.iter().enumerate() {
                let (site, coefficients) = kernel.stencil(path, index as i32 + 1)?;
                for (charge, coefficient) in coefficients.into_iter().enumerate() {
                    if coefficient.re.is_zero() && coefficient.im.is_zero() {
                        continue;
                    }
                    let mut output = *path;
                    output[site] = charge as u8;
                    let row = QUTRIT_PATHS
                        .iter()
                        .position(|p| *p == output)
                        .ok_or("exchange escaped the complete qutrit fusion sector")?;
                    generator.cells[3 * row + column] = coefficient;
                }
            }
        }
        Ok(Self {
            format: kernel.format().clone(),
            generators,
        })
    }
    pub fn format(&self) -> &FixedPointFormat {
        &self.format
    }
    pub fn generator(&self, index: i32) -> Result<QutritMatrix, String> {
        let magnitude = index.unsigned_abs();
        if !(1..=3).contains(&magnitude) {
            return Err("qutrit exchange index must be ±1, ±2, or ±3".into());
        }
        let gate = &self.generators[magnitude as usize - 1];
        Ok(if index < 0 {
            gate.adjoint()
        } else {
            gate.clone()
        })
    }
    pub fn evaluate(&self, word: &[i32]) -> Result<QutritMatrix, String> {
        let mut result = QutritMatrix::identity(&self.format);
        for &index in word {
            result = self.generator(index)?.multiply(&result, &self.format);
        }
        Ok(result)
    }
}

pub struct QutritCarrier {
    format: FixedPointFormat,
    amplitudes: [FixedComplex; 3],
}
impl QutritCarrier {
    pub fn basis(algebra: &FibonacciQutrit, value: Trit) -> Self {
        Self {
            format: algebra.format.clone(),
            amplitudes: core::array::from_fn(|i| {
                if i == value as usize {
                    FixedComplex {
                        re: algebra.format.scale(),
                        im: BigInt::zero(),
                    }
                } else {
                    zero()
                }
            }),
        }
    }
    pub fn amplitudes(&self) -> &[FixedComplex; 3] {
        &self.amplitudes
    }
    pub fn exchange(&mut self, algebra: &FibonacciQutrit, index: i32) -> Result<(), String> {
        if self.format != algebra.format {
            return Err("qutrit fixed-point formats differ".into());
        }
        let gate = algebra.generator(index)?;
        let output: [FixedComplex; 3] = core::array::from_fn(|row| {
            let mut value = zero();
            for column in 0..3 {
                let term = gate.cells[3 * row + column].mul(&self.amplitudes[column], &self.format);
                value.re += term.re;
                value.im += term.im;
            }
            value
        });
        if output.iter().all(|z| z.re.is_zero() && z.im.is_zero()) {
            return Err("qutrit exchange produced zero amplitude".into());
        }
        self.amplitudes = output;
        Ok(())
    }
    pub fn born_masses(&self) -> [BigUint; 3] {
        core::array::from_fn(|i| {
            let z = &self.amplitudes[i];
            (&z.re * &z.re + &z.im * &z.im).to_biguint().unwrap()
        })
    }
    pub fn measure<F>(&mut self, entropy: F) -> Result<Trit, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let index = sample_born_masses(&self.born_masses(), entropy)?;
        for (i, value) in self.amplitudes.iter_mut().enumerate() {
            if i != index {
                *value = zero();
            }
        }
        Trit::try_from(index as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qutrit_braid_and_three_outcome_readout_at_required_source_widths() {
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: alloc::vec::Vec<_> = line.split('\t').collect();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let algebra = FibonacciQutrit::new(&source).unwrap();
            let tolerance = BigUint::from(256u16);
            for index in 1..=3 {
                assert!(
                    algebra
                        .generator(index)
                        .unwrap()
                        .unitarity_residual(algebra.format())
                        < tolerance
                );
                let identity = algebra.evaluate(&[index, -index]).unwrap();
                assert!(
                    identity.maximum_difference(&QutritMatrix::identity(algebra.format()))
                        < tolerance
                );
            }
            for (a, b) in [(1, 2), (2, 3)] {
                assert!(
                    algebra
                        .evaluate(&[a, b, a])
                        .unwrap()
                        .maximum_difference(&algebra.evaluate(&[b, a, b]).unwrap())
                        < tolerance
                );
            }
            assert!(
                algebra
                    .evaluate(&[1, 3])
                    .unwrap()
                    .maximum_difference(&algebra.evaluate(&[3, 1]).unwrap())
                    < tolerance
            );
            for value in [Trit::Zero, Trit::One, Trit::Two] {
                let mut state = QutritCarrier::basis(&algebra, value);
                assert_eq!(
                    state
                        .measure(|_| Err("deterministic outcome needs no entropy".into()))
                        .unwrap(),
                    value
                );
            }
            let mut state = QutritCarrier::basis(&algebra, Trit::Zero);
            for index in [2, 3] {
                state.exchange(&algebra, index).unwrap();
            }
            let masses = state.born_masses();
            assert!(masses.iter().all(|mass| !mass.is_zero()));
            // Select the third interval explicitly, exercising all three masses.
            let draw = &masses[0] + &masses[1];
            assert_eq!(
                state
                    .measure(|bytes| {
                        bytes.fill(0);
                        let value = draw.to_bytes_le();
                        bytes[..value.len()].copy_from_slice(&value);
                        Ok(())
                    })
                    .unwrap(),
                Trit::Two
            );
            assert!(state.born_masses()[0].is_zero());
            assert!(state.born_masses()[1].is_zero());
            assert!(!state.born_masses()[2].is_zero());
            assert!(algebra.generator(4).is_err());
            println!(
                "{}-bit source: three-channel braid relations and outcome 2 retained",
                source.bits()
            );
        }
    }
}
