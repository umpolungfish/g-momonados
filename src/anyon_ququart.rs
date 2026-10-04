//! Full four-channel carrier in the six-anyon vacuum sector.
//! The fifth fusion channel is retained throughout exchange and readout.
use crate::anyon_fusion_kernel::sample_born_masses;
use crate::anyon_pair::{FibonacciPair, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat, PhaseReadoutAccumulator};
use alloc::string::String;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

fn zero() -> FixedComplex {
    FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }
}
fn mass(z: &FixedComplex) -> BigUint {
    (&z.re * &z.re + &z.im * &z.im).to_biguint().unwrap()
}
fn conjugate(z: &FixedComplex) -> FixedComplex {
    FixedComplex {
        re: z.re.clone(),
        im: -&z.im,
    }
}

/// Base-four readout, ordered T,F,t,f. These are phase digits emitted by a
/// four-pole measurement; SIC outcomes are indexed by SixteenOutcome instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum QuquartDigit {
    T = 0,
    F = 1,
    InfoT = 2,
    InfoF = 3,
}
impl TryFrom<u8> for QuquartDigit {
    type Error = String;
    fn try_from(n: u8) -> Result<Self, String> {
        match n {
            0 => Ok(Self::T),
            1 => Ok(Self::F),
            2 => Ok(Self::InfoT),
            3 => Ok(Self::InfoF),
            _ => Err("ququart digit must fit two bits".into()),
        }
    }
}

#[derive(Default)]
pub struct QuquartPhaseReadout {
    digits: usize,
    numerator: BigUint,
}
impl QuquartPhaseReadout {
    pub fn push(&mut self, digit: QuquartDigit) -> Result<(), String> {
        let shift = self
            .digits
            .checked_mul(2)
            .ok_or("ququart phase width overflow")?;
        let count = self
            .digits
            .checked_add(1)
            .ok_or("ququart phase width overflow")?;
        self.numerator += BigUint::from(digit as u8) << shift;
        self.digits = count;
        Ok(())
    }
    pub fn numerator(&self) -> &BigUint {
        &self.numerator
    }
    pub fn denominator(&self) -> Result<BigUint, String> {
        Ok(BigUint::one()
            << self
                .digits
                .checked_mul(2)
                .ok_or("ququart phase width overflow")?)
    }
    pub fn close(
        &self,
        evidence: &mut PhaseReadoutAccumulator,
        source: &BigUint,
        base: &BigUint,
    ) -> Result<Option<(BigUint, BigUint, BigUint)>, String> {
        if self.digits == 0 || source.bits() < 128 {
            return Err(
                "ququart factor closure needs phase digits and a source of at least 128 bits"
                    .into(),
            );
        }
        evidence.close_fraction(&self.numerator, &self.denominator()?, base, source)
    }
}

pub enum QuquartSicOutcome {
    Carrier(crate::sic::SixteenOutcome),
    OutsideCarrier,
}

/// Dimension-four analytic fiducial evaluated through integer square roots.
/// No floating-point amplitudes enter the fixed-point measurement path.
pub struct FixedQuquartSic {
    format: FixedPointFormat,
    rays: [[FixedComplex; 4]; 16],
}
impl FixedQuquartSic {
    pub fn new(format: &FixedPointFormat) -> Result<Self, String> {
        let scale = format.scale();
        let sqrt =
            |x: BigInt| -> BigInt { BigInt::from((x * &scale).to_biguint().unwrap().sqrt()) };
        let mul = |x: &BigInt, y: &BigInt| -> BigInt { (x * y) / &scale };
        let root2 = sqrt(&scale * 2u8);
        let root5 = sqrt(&scale * 5u8);
        let a = sqrt((&scale * 5u8 - &root5) / 40u8);
        let c = sqrt(&scale * 2u8 + &root2) / 2u8;
        let s = sqrt(&scale * 2u8 - &root2) / 2u8;
        let u = sqrt(&scale * 2u8 + &root5);
        let ray = [
            FixedComplex {
                re: mul(&a, &c) * 2u8,
                im: BigInt::zero(),
            },
            FixedComplex {
                re: mul(&a, &mul(&(&scale - &u), &s)),
                im: mul(&a, &mul(&(&scale + &u), &c)),
            },
            FixedComplex {
                re: BigInt::zero(),
                im: mul(&a, &s) * 2u8,
            },
            FixedComplex {
                re: mul(&a, &mul(&(&scale + &u), &s)),
                im: mul(&a, &mul(&(&scale - &u), &c)),
            },
        ];
        let mut rays = core::array::from_fn(|_| core::array::from_fn(|_| zero()));
        for (i, output) in rays.iter_mut().enumerate() {
            let p = i / 4;
            let q = i % 4;
            for (n, z) in output.iter_mut().enumerate() {
                let shifted = (n + 4 - p) % 4;
                // tau=-exp(pi i/4), omega=exp(pi i/2), D=tau^pq X^p Z^q.
                let turns = (5 * p * q + 2 * q * shifted) % 8;
                let phase = FixedComplex::winding_twiddle(
                    &BigInt::from(turns),
                    &BigUint::from(8u8),
                    format,
                )?;
                *z = phase.mul(&ray[shifted], format);
            }
        }
        Ok(Self {
            format: format.clone(),
            rays,
        })
    }
    pub fn rays(&self) -> &[[FixedComplex; 4]; 16] {
        &self.rays
    }
}

pub struct QuquartCarrier {
    format: FixedPointFormat,
    amplitudes: [FixedComplex; 5],
}
impl QuquartCarrier {
    pub fn basis(algebra: &FibonacciPair, digit: QuquartDigit) -> Self {
        let channel = COMPUTATIONAL_CHANNELS[digit as usize];
        Self {
            format: algebra.format().clone(),
            amplitudes: core::array::from_fn(|i| {
                if i == channel {
                    FixedComplex {
                        re: algebra.format().scale(),
                        im: BigInt::zero(),
                    }
                } else {
                    zero()
                }
            }),
        }
    }
    pub fn amplitudes(&self) -> &[FixedComplex; 5] {
        &self.amplitudes
    }
    pub fn exchange(&mut self, algebra: &FibonacciPair, index: i32) -> Result<(), String> {
        if self.format != *algebra.format() {
            return Err("ququart exchange format mismatch".into());
        }
        let gate = algebra.in_pair_channels(&algebra.evaluate(&[index])?);
        let output = core::array::from_fn(|row| {
            let mut z = zero();
            for col in 0..5 {
                let term = gate.0[5 * row + col].mul(&self.amplitudes[col], &self.format);
                z.re += term.re;
                z.im += term.im;
            }
            z
        });
        self.amplitudes = output;
        Ok(())
    }
    /// Sixteen SIC masses plus the retained outside-carrier channel. The
    /// latter is not identified with either informational basis atom.
    pub fn sic_masses(&self, sic: &FixedQuquartSic) -> Result<[BigUint; 17], String> {
        if self.format != sic.format {
            return Err("ququart SIC format mismatch".into());
        }
        Ok(core::array::from_fn(|i| {
            if i == 16 {
                return mass(&self.amplitudes[LEAKAGE_CHANNEL]) * 4u8;
            }
            let mut overlap = zero();
            for (j, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let z = conjugate(&sic.rays[i][j]).mul(&self.amplitudes[channel], &self.format);
                overlap.re += z.re;
                overlap.im += z.im;
            }
            mass(&overlap)
        }))
    }
    pub fn measure_sic<F>(
        &mut self,
        sic: &FixedQuquartSic,
        entropy: F,
    ) -> Result<QuquartSicOutcome, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let i = sample_born_masses(&self.sic_masses(sic)?, entropy)?;
        if i == 16 {
            for &j in &COMPUTATIONAL_CHANNELS {
                self.amplitudes[j] = zero();
            }
            return Ok(QuquartSicOutcome::OutsideCarrier);
        }
        self.amplitudes = core::array::from_fn(|_| zero());
        for (j, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            self.amplitudes[channel] = sic.rays[i][j].clone();
        }
        Ok(QuquartSicOutcome::Carrier(
            crate::sic::SixteenOutcome::new(i as u8).map_err(|e| e.to_string())?,
        ))
    }
    pub fn measure_phase_digit<F>(&mut self, entropy: F) -> Result<Option<QuquartDigit>, String>
    where
        F: FnMut(&mut [u8]) -> Result<(), String>,
    {
        let masses: [BigUint; 5] = core::array::from_fn(|i| mass(&self.amplitudes[i]));
        let index = sample_born_masses(&masses, entropy)?;
        for (i, z) in self.amplitudes.iter_mut().enumerate() {
            if i != index {
                *z = zero();
            }
        }
        Ok(COMPUTATIONAL_CHANNELS
            .iter()
            .position(|j| *j == index)
            .map(|j| QuquartDigit::try_from(j as u8).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Signed;
    #[test]
    fn four_phase_digits_keep_leading_zero_and_resolution() {
        let mut readout = QuquartPhaseReadout::default();
        for digit in [
            QuquartDigit::T,
            QuquartDigit::F,
            QuquartDigit::InfoT,
            QuquartDigit::InfoF,
        ] {
            readout.push(digit).unwrap();
        }
        assert_eq!(readout.numerator(), &BigUint::from(228u16));
        assert_eq!(readout.denominator().unwrap(), BigUint::from(256u16));
    }
    #[test]
    fn ququart_fixed_sic_and_exchange_at_all_required_source_widths() {
        for line in include_str!("../measurements/anyon-extractor-width-controls.tsv")
            .lines()
            .skip(1)
        {
            let fields: alloc::vec::Vec<_> = line.split('\t').collect();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            let algebra = FibonacciPair::new(&source).unwrap();
            let sic = FixedQuquartSic::new(algebra.format()).unwrap();
            let scale = algebra.format().scale();
            let square = (&scale * &scale).to_biguint().unwrap();
            for ray in sic.rays() {
                let norm: BigUint = ray.iter().map(mass).sum();
                assert!((BigInt::from(norm) - BigInt::from(square.clone())).abs() < &scale * 128u8);
            }
            for i in 0..16 {
                for j in 0..i {
                    let mut overlap = zero();
                    for k in 0..4 {
                        let z = conjugate(&sic.rays[i][k]).mul(&sic.rays[j][k], algebra.format());
                        overlap.re += z.re;
                        overlap.im += z.im;
                    }
                    assert!(
                        (BigInt::from(mass(&overlap)) * 5u8 - BigInt::from(square.clone())).abs()
                            < &scale * 1024u16
                    );
                }
            }
            for digit in [
                QuquartDigit::T,
                QuquartDigit::F,
                QuquartDigit::InfoT,
                QuquartDigit::InfoF,
            ] {
                let mut state = QuquartCarrier::basis(&algebra, digit);
                assert_eq!(
                    state
                        .measure_phase_digit(|_| Err("single outcome requires no entropy".into()))
                        .unwrap(),
                    Some(digit)
                );
            }
            let original = QuquartCarrier::basis(&algebra, QuquartDigit::InfoF);
            let masses = original.sic_masses(&sic).unwrap();
            assert!(masses[..16].iter().all(|m| !m.is_zero()));
            assert!(masses[16].is_zero());
            for i in 0..16 {
                let draw: BigUint = masses[..i].iter().cloned().sum();
                let mut state = QuquartCarrier::basis(&algebra, QuquartDigit::InfoF);
                let outcome = state
                    .measure_sic(&sic, |bytes| {
                        bytes.fill(0);
                        let value = draw.to_bytes_le();
                        bytes[..value.len()].copy_from_slice(&value);
                        Ok(())
                    })
                    .unwrap();
                assert!(matches!(outcome,QuquartSicOutcome::Carrier(a) if a.mask()==i as u8));
            }
            let mut state = QuquartCarrier::basis(&algebra, QuquartDigit::T);
            state.exchange(&algebra, 3).unwrap();
            assert!(!mass(&state.amplitudes[LEAKAGE_CHANNEL]).is_zero());
            state.exchange(&algebra, -3).unwrap();
            let start = QuquartCarrier::basis(&algebra, QuquartDigit::T);
            for (a, b) in state.amplitudes.iter().zip(start.amplitudes.iter()) {
                assert!((&a.re - &b.re).abs() < BigInt::from(4096u16));
                assert!((&a.im - &b.im).abs() < BigInt::from(4096u16));
            }
            println!(
                "{}-bit semiprime: fixed WH(4), sixteen SIC outcomes, four phase digits",
                source.bits()
            );
        }
    }
}
