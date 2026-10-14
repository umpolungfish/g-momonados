//! Full four-channel carrier in the six-anyon vacuum sector.
//! The fifth fusion channel is retained throughout exchange and readout.
use crate::anyon_fusion_kernel::sample_born_masses;
use crate::anyon_pair::{FibonacciPair, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat, PhaseReadoutAccumulator};
use alloc::string::String;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, Zero};
use crate::word_tape::{SignedWordTape, WordTape};

fn zero() -> FixedComplex {
    FixedComplex {
        re: BigInt::zero(),
        im: BigInt::zero(),
    }
}
fn mass(z: &FixedComplex) -> BigUint {
    let re = SignedWordTape::from_bigint(&z.re);
    let im = SignedWordTape::from_bigint(&z.im);
    re.magnitude().mul(re.magnitude()).add(&im.magnitude().mul(im.magnitude())).to_biguint()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuquartSicOutcome {
    Carrier(crate::sic::SixteenOutcome),
    OutsideCarrier,
}

#[derive(Clone)]
pub struct SicPhaseEvidence {
    sic: FixedQuquartSic,
    hypotheses: alloc::vec::Vec<(BigUint, BigUint, BigUint)>,
    observations: usize,
}

impl SicPhaseEvidence {
    pub fn new(format: &FixedPointFormat) -> Result<Self, String> {
        Ok(Self {
            sic: FixedQuquartSic::new(format)?,
            hypotheses: alloc::vec::Vec::new(),
            observations: 0,
        })
    }

    pub fn add_hypothesis(
        &mut self,
        numerator: BigUint,
        denominator: BigUint,
    ) -> Result<(), String> {
        if denominator.is_zero() {
            return Err("SIC phase hypothesis denominator is zero".into());
        }
        let mut numerator = numerator % &denominator;
        let mut a = numerator.clone();
        let mut b = denominator.clone();
        while !b.is_zero() {
            let remainder = &a % &b;
            a = b;
            b = remainder;
        }
        if !a.is_zero() {
            numerator /= &a;
            let denominator = denominator / a;
            return self.insert_hypothesis(numerator, denominator);
        }
        self.insert_hypothesis(BigUint::zero(), BigUint::one())
    }

    pub fn validate_source(&self, source: &BigUint) -> Result<(), String> {
        if self.sic.format != FixedPointFormat::for_modulus(source)? {
            return Err("SIC phase evidence precision differs from its source".into());
        }
        Ok(())
    }

    fn insert_hypothesis(
        &mut self,
        numerator: BigUint,
        denominator: BigUint,
    ) -> Result<(), String> {
        if self.hypotheses.iter().any(|(known, known_denominator, _)| {
            known == &numerator && known_denominator == &denominator
        }) {
            return Err("duplicate SIC phase hypothesis".into());
        }
        self.hypotheses
            .push((numerator, denominator, BigUint::one()));
        Ok(())
    }

    pub fn observe(
        &mut self,
        outcome: QuquartSicOutcome,
        power: &BigUint,
        feedback_numerator: &BigUint,
        feedback_denominator: &BigUint,
    ) -> Result<(), String> {
        if self.hypotheses.is_empty() {
            return Err("SIC phase evidence has no candidate hypotheses".into());
        }
        let next_observation_count = self
            .observations
            .checked_add(1)
            .ok_or("SIC observation count overflow")?;
        let mask = match outcome {
            QuquartSicOutcome::Carrier(outcome) => outcome.mask() as usize,
            QuquartSicOutcome::OutsideCarrier => {
                return Err("outside-carrier event is incompatible with phase hypotheses".into())
            }
        };
        for (numerator, denominator, weight) in &mut self.hypotheses {
            let likelihood = self.sic.phase_outcome_masses(
                numerator,
                denominator,
                power,
                feedback_numerator,
                feedback_denominator,
            )?[mask]
                .clone();
            *weight *= likelihood;
        }
        self.observations = next_observation_count;
        Ok(())
    }

    pub fn scores(&self) -> impl Iterator<Item = (&BigUint, &BigUint, &BigUint)> {
        self.hypotheses
            .iter()
            .map(|(numerator, denominator, weight)| (numerator, denominator, weight))
    }

    pub fn observation_count(&self) -> usize {
        self.observations
    }
}

/// Dimension-four analytic fiducial evaluated through integer square roots.
/// No floating-point amplitudes enter the fixed-point measurement path.
#[derive(Clone)]
pub struct FixedQuquartSic {
    format: FixedPointFormat,
    rays: [[FixedComplex; 4]; 16],
}

/// Complete dual synthesis in the shared-work convention <work_i|work_j>.
/// Residuals and tolerance are integers in the original Gram units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SicGramReconstruction {
    pub recovered: [(BigInt, BigInt); 16],
    pub maximum_residual: BigInt,
    pub tolerance: BigInt,
}

/// Rational dual synthesis before terminal integer rounding. The signed
/// remainders retain the finite-precision analytic frame's actual discrepancy.
pub struct SicRationalReconstruction {
    pub rounded: SicGramReconstruction,
    pub denominator: BigInt,
    pub signed_residual_numerators: [(BigInt, BigInt); 16],
}

impl FixedQuquartSic {
    /// All sixteen projector masses from a shared work Gram matrix. The POVM
    /// effect masses are these values divided by four.
    pub fn gram_masses(&self, gram: &[(BigInt,BigInt);16]) -> Result<[BigUint;16],String> {
        let denominator = WordTape::one_at(2 * self.format.w_bits as usize);
        Ok(self.gram_mass_words(gram)?.map(|mass| mass.divmod(&denominator).unwrap().0.to_biguint()))
    }

    /// Projector masses with common denominator scale², retaining every
    /// remainder until the complete analytic dual frame has acted.
    pub fn gram_mass_numerators(&self, gram: &[(BigInt,BigInt);16]) -> Result<[BigUint;16],String> {
        Ok(self.gram_mass_words(gram)?.map(|mass| mass.to_biguint()))
    }

    fn gram_mass_words(&self, gram: &[(BigInt,BigInt);16]) -> Result<[WordTape;16],String> {
        let gram = (*gram).clone().map(|(ref re, ref im)|
            (SignedWordTape::from_bigint(re), SignedWordTape::from_bigint(im)));
        let mut masses = core::array::from_fn(|_| WordTape::zero());
        for (mass,ray) in masses.iter_mut().zip(&self.rays) {
            let ray = ray.clone().map(|z|
                (SignedWordTape::from_bigint(&z.re), SignedWordTape::from_bigint(&z.im)));
            let mut real = SignedWordTape::zero();
            for i in 0..4 {
                for j in 0..4 {
                    let coefficient_re = ray[i].0.mul(&ray[j].0).add(&ray[i].1.mul(&ray[j].1));
                    let coefficient_im = ray[i].1.mul(&ray[j].0).sub(&ray[i].0.mul(&ray[j].1));
                    let (gr,gi) = &gram[4*i+j];
                    real = real.add(&coefficient_re.mul(gr).sub(&coefficient_im.mul(gi)));
                }
            }
            if real.is_negative() { return Err("negative SIC Gram mass".into()); }
            *mass = real.magnitude().clone();
        }
        Ok(masses)
    }

    /// Full mass-only dual reconstruction: the negative trace correction is
    /// recovered from sum(m_i)/4 rather than supplied from the reference Gram.
    /// Only the completed synthesis is rounded to the nearest integer.
    pub fn certify_gram_frame_unrounded_masses(&self,
        gram: &[(BigInt, BigInt); 16], numerators: &[BigUint; 16])
        -> Result<SicRationalReconstruction, String> {
        let gram_words = (*gram).clone().map(|(re, im)|
            (SignedWordTape::from_bigint(&re), SignedWordTape::from_bigint(&im)));
        // Retain the established Hermitian/population and frame checks.
        for row in 0..4 {
            if !gram_words[5*row].1.is_zero() || gram_words[5*row].0.is_negative() {
                return Err("invalid control Gram diagonal".into());
            }
            for col in 0..row {
                if gram_words[4*row+col].0 != gram_words[4*col+row].0
                    || gram_words[4*row+col].1 != gram_words[4*col+row].1.negate() {
                    return Err("control Gram is not Hermitian".into());
                }
            }
        }
        let scale_squared = SignedWordTape::from_magnitude(
            WordTape::one_at(2 * self.format.w_bits as usize), false);
        let denominator = scale_squared.mul(&scale_squared).shl(2);
        let masses = (*numerators).clone().map(|ref mass| SignedWordTape::from_magnitude(WordTape::from_biguint(mass), false));
        let total = masses.iter().fold(SignedWordTape::zero(), |sum, mass| sum.add(mass));
        let rays = self.rays.clone().map(|ray| ray.map(|z|
            (SignedWordTape::from_bigint(&z.re), SignedWordTape::from_bigint(&z.im))));
        let five = SignedWordTape::from_small(5);
        let recovered_numerators: [(SignedWordTape, SignedWordTape); 16] = core::array::from_fn(|entry| {
            let row = entry / 4;
            let col = entry % 4;
            let mut real = SignedWordTape::zero();
            let mut imaginary = SignedWordTape::zero();
            for (ray, mass) in rays.iter().zip(&masses) {
                real = real.add(&mass.mul(&ray[row].0.mul(&ray[col].0).add(&ray[row].1.mul(&ray[col].1))));
                imaginary = imaginary.add(&mass.mul(&ray[row].0.mul(&ray[col].1).sub(&ray[row].1.mul(&ray[col].0))));
            }
            let correction = if row == col { total.mul(&scale_squared) } else { SignedWordTape::zero() };
            (real.mul(&five).sub(&correction), imaginary.mul(&five))
        });
        let round = |value: &SignedWordTape| {
            let magnitude = value.abs().add(&denominator.shr(1)).div(&denominator);
            if value.is_negative() { magnitude.negate() } else { magnitude }
        };
        let recovered: [(SignedWordTape, SignedWordTape);16] = core::array::from_fn(|i| (
            round(&recovered_numerators[i].0), round(&recovered_numerators[i].1)));
        let signed_residual_numerators: [(SignedWordTape, SignedWordTape);16] = core::array::from_fn(|i| (
            recovered_numerators[i].0.sub(&gram_words[i].0.mul(&denominator)),
            recovered_numerators[i].1.sub(&gram_words[i].1.mul(&denominator))));
        let trace = (0..4).fold(SignedWordTape::zero(), |sum, i| sum.add(&gram_words[5*i].0));
        let tolerance = core::cmp::max(SignedWordTape::from_small(1), trace.shr((self.format.w_bits/2) as usize));
        let maximum_residual = recovered.iter().zip(&gram_words).map(|((re,im),(gr,gi))|
            re.sub(gr).abs().max(im.sub(gi).abs())).max().unwrap();
        if maximum_residual > tolerance {
            return Err("unrounded-mass SIC dual reconstruction differs from the full control Gram".into());
        }
        // Certify the rational error too: terminal rounding cannot conceal
        // a discrepancy larger than the existing tolerance plus half a unit.
        let limit_twice = tolerance.shl(1).add(&SignedWordTape::from_small(1)).mul(&denominator);
        if signed_residual_numerators.iter().any(|(re, im)|
            re.abs().shl(1) > limit_twice || im.abs().shl(1) > limit_twice) {
            return Err("SIC rational residual exceeds the terminal rounding tolerance".into());
        }
        Ok(SicRationalReconstruction { rounded: SicGramReconstruction {
            recovered: recovered.map(|(re, im)| (re.to_bigint(), im.to_bigint())),
            maximum_residual: maximum_residual.to_bigint(), tolerance: tolerance.to_bigint() },
            denominator: denominator.to_bigint(),
            signed_residual_numerators: signed_residual_numerators.map(|(re, im)| (re.to_bigint(), im.to_bigint())) })
    }

    /// Certify all populations and coherences from the sixteen projector masses.
    /// For d=4, synthesis is (5/4) sum_i m_i P_i - trace I.
    /// ZetaSICDualFrame.ququart_coefficients checks 5/4 = 1-3*zeta(-1)
    /// and the inverse-Gram uniform correction -1/16 = (3/4)*zeta(-1).
    pub fn certify_gram_frame(&self, gram: &[(BigInt,BigInt);16], masses: &[BigUint;16]) -> Result<SicGramReconstruction,String> {
        let trace: BigInt = (0..4).map(|i| gram[5*i].0.clone()).sum();
        if trace < BigInt::zero() { return Err("negative control Gram trace".into()); }
        for row in 0..4 {
            if !gram[5*row].1.is_zero() || gram[5*row].0 < BigInt::zero() {
                return Err("invalid control Gram diagonal".into());
            }
            for col in 0..row {
                if gram[4*row+col].0 != gram[4*col+row].0
                    || gram[4*row+col].1 != -&gram[4*col+row].1 {
                    return Err("control Gram is not Hermitian".into());
                }
            }
        }
        let scale_squared = self.format.scale().pow(2);
        // The fixed analytic frame has finite integer rounding. Compare with
        // a source-dependent tolerance far below a computational Born digit.
        let tolerance = core::cmp::max(BigInt::from(1u8), &trace >> (self.format.w_bits/2) as usize);
        let denominator = &scale_squared * 4u8;
        let recovered: [(BigInt,BigInt);16] = core::array::from_fn(|entry| {
            let row = entry / 4;
            let col = entry % 4;
            let mut real = BigInt::zero();
            let mut imaginary = BigInt::zero();
            for (ray, mass) in self.rays.iter().zip(masses) {
                let mass = BigInt::from(mass.clone());
                real += &mass * (&ray[row].re*&ray[col].re + &ray[row].im*&ray[col].im);
                // control_gram stores <work_row|work_col>, the conjugate
                // of the usual density-matrix entry.
                imaginary += mass * (&ray[row].re*&ray[col].im - &ray[row].im*&ray[col].re);
            }
            let diagonal = if row == col { trace.clone() } else { BigInt::zero() };
            (real*5u8 / &denominator - diagonal, imaginary*5u8 / &denominator)
        });
        let maximum_residual = recovered.iter().zip(gram).map(|((re,im),(gr,gi))| {
            (re-gr).abs().max((im-gi).abs())
        }).max().unwrap();
        if maximum_residual > tolerance {
            return Err("SIC dual reconstruction differs from the full control Gram".into());
        }
        Ok(SicGramReconstruction { recovered, maximum_residual, tolerance })
    }

    pub fn validate_gram_frame(&self, gram: &[(BigInt,BigInt);16], masses: &[BigUint;16]) -> Result<(),String> {
        self.certify_gram_frame(gram, masses).map(|_| ())
    }

    pub fn new(format: &FixedPointFormat) -> Result<Self, String> {
        let scale = SignedWordTape::from_magnitude(WordTape::one_at(format.w_bits as usize), false);
        let two = SignedWordTape::from_small(2);
        let five = SignedWordTape::from_small(5);
        let sqrt = |x: SignedWordTape| -> SignedWordTape {
            assert!(!x.is_negative(), "analytic SIC square root requires nonnegative support");
            SignedWordTape::from_magnitude(x.mul(&scale).magnitude().isqrt(), false)
        };
        let mul = |x: &SignedWordTape, y: &SignedWordTape| x.mul(y).div(&scale);
        let root2 = sqrt(scale.mul(&two));
        let root5 = sqrt(scale.mul(&five));
        let a = sqrt(scale.mul(&five).sub(&root5).div(&SignedWordTape::from_small(40)));
        let c = sqrt(scale.mul(&two).add(&root2)).div(&two);
        let s = sqrt(scale.mul(&two).sub(&root2)).div(&two);
        let u = sqrt(scale.mul(&two).add(&root5));
        let ray = [
            FixedComplex {
                re: mul(&a, &c).mul(&two).to_bigint(),
                im: BigInt::zero(),
            },
            FixedComplex {
                re: mul(&a, &mul(&scale.sub(&u), &s)).to_bigint(),
                im: mul(&a, &mul(&scale.add(&u), &c)).to_bigint(),
            },
            FixedComplex {
                re: BigInt::zero(),
                im: mul(&a, &s).mul(&two).to_bigint(),
            },
            FixedComplex {
                re: mul(&a, &mul(&scale.add(&u), &s)).to_bigint(),
                im: mul(&a, &mul(&scale.sub(&u), &c)).to_bigint(),
            },
        ];
        let mut rays = core::array::from_fn(|_| core::array::from_fn(|_| zero()));
        let phase_numerators: [BigInt; 8] = core::array::from_fn(|turn| BigInt::from(turn));
        let phases = FixedComplex::winding_twiddle_table(
            &phase_numerators,
            &BigUint::from(8u8),
            format,
        )?;
        for (i, output) in rays.iter_mut().enumerate() {
            let p = i / 4;
            let q = i % 4;
            for (n, z) in output.iter_mut().enumerate() {
                let shifted = (n + 4 - p) % 4;
                // tau=-exp(pi i/4), omega=exp(pi i/2), D=tau^pq X^p Z^q.
                let turns = (5 * p * q + 2 * q * shifted) % 8;
                *z = phases[turns].mul(&ray[shifted], format);
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

    pub fn phase_outcome_masses(
        &self,
        phase_numerator: &BigUint,
        phase_denominator: &BigUint,
        power: &BigUint,
        feedback_numerator: &BigUint,
        feedback_denominator: &BigUint,
    ) -> Result<[BigUint; 16], String> {
        if phase_denominator.is_zero() || feedback_denominator.is_zero() {
            return Err("SIC phase likelihood requires nonzero denominators".into());
        }
        let mut control: [FixedComplex; 4] = core::array::from_fn(|_| zero());
        for (basis, amplitude) in control.iter_mut().enumerate() {
            let mut sum = zero();
            for input in 0..4 {
                let powered_phase = FixedComplex::winding_twiddle(
                    &BigInt::from(phase_numerator * power * input),
                    phase_denominator,
                    &self.format,
                )?;
                let feedback_phase = FixedComplex::winding_twiddle(
                    &-BigInt::from(feedback_numerator * input),
                    feedback_denominator,
                    &self.format,
                )?;
                let fourier_phase = FixedComplex::winding_twiddle(
                    &-BigInt::from(basis * input),
                    &BigUint::from(4u8),
                    &self.format,
                )?;
                let term = powered_phase
                    .mul(&feedback_phase, &self.format)
                    .mul(&fourier_phase, &self.format);
                sum.re += term.re;
                sum.im += term.im;
            }
            amplitude.re = sum.re / 4u8;
            amplitude.im = sum.im / 4u8;
        }
        Ok(core::array::from_fn(|outcome| {
            let mut overlap = zero();
            for (component, amplitude) in control.iter().enumerate() {
                let conjugate = FixedComplex {
                    re: self.rays[outcome][component].re.clone(),
                    im: -&self.rays[outcome][component].im,
                };
                let term = conjugate.mul(amplitude, &self.format);
                overlap.re += term.re;
                overlap.im += term.im;
            }
            mass(&overlap)
        }))
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
    /// Native Z4 Fourier target. A braid backend must synthesize this complete
    /// four-channel operator; applying H to one lane does not implement it.
    pub fn fourier_target(&mut self, inverse: bool) {
        let old = &self.amplitudes;
        let values: [FixedComplex; 4] = core::array::from_fn(|k| {
            let mut sum = zero();
            for (l, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let z = &old[channel];
                let phase = if inverse {
                    (4 - (k * l) % 4) % 4
                } else {
                    (k * l) % 4
                };
                let term = match phase {
                    0 => z.clone(),
                    1 => FixedComplex {
                        re: -&z.im,
                        im: z.re.clone(),
                    },
                    2 => FixedComplex {
                        re: -&z.re,
                        im: -&z.im,
                    },
                    _ => FixedComplex {
                        re: z.im.clone(),
                        im: -&z.re,
                    },
                };
                sum.re += term.re;
                sum.im += term.im;
            }
            FixedComplex {
                re: sum.re / 2u8,
                im: sum.im / 2u8,
            }
        });
        for (k, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            self.amplitudes[channel] = values[k].clone();
        }
    }
    /// exp(-2 pi i k numerator/4^m), for all four computational channels.
    pub fn feedback_target(&mut self, numerator: &BigUint, m: usize) -> Result<(), String> {
        let bits = m
            .checked_mul(2)
            .ok_or("ququart feedback denominator overflow")?;
        let denominator = BigUint::one() << bits;
        let mut phases: [FixedComplex; 4] = core::array::from_fn(|_| zero());
        for (k, z) in phases.iter_mut().enumerate() {
            *z = FixedComplex::winding_twiddle(
                &-BigInt::from(numerator * k),
                &denominator,
                &self.format,
            )?;
        }
        for (k, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            self.amplitudes[channel] = phases[k].mul(&self.amplitudes[channel], &self.format);
        }
        Ok(())
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
    fn word_sic_dual_preserves_every_signed_rational_residual() {
        let format = FixedPointFormat::for_modulus_bits(20).unwrap();
        let frame = FixedQuquartSic::new(&format).unwrap();
        let work = [[(1i32,0i32),(0,2)], [(2,1),(1,0)], [(-1,2),(3,-1)], [(0,-1),(2,2)]];
        let gram: [(BigInt,BigInt);16] = core::array::from_fn(|entry| {
            let (row,col) = (entry/4,entry%4);
            let mut re = BigInt::zero(); let mut im = BigInt::zero();
            for component in 0..2 {
                let (ar,ai) = work[row][component]; let (br,bi) = work[col][component];
                re += BigInt::from(ar*br+ai*bi); im += BigInt::from(ar*bi-ai*br);
            }
            (re,im)
        });
        // Host arithmetic is confined to this independent comparison instrument.
        let expected_masses: [BigUint;16] = core::array::from_fn(|i| {
            let ray = &frame.rays[i]; let mut value = BigInt::zero();
            for row in 0..4 { for col in 0..4 {
                let cr = &ray[row].re*&ray[col].re + &ray[row].im*&ray[col].im;
                let ci = &ray[row].im*&ray[col].re - &ray[row].re*&ray[col].im;
                value += cr*&gram[4*row+col].0-ci*&gram[4*row+col].1;
            }}
            value.to_biguint().unwrap()
        });
        let actual_masses = frame.gram_mass_numerators(&gram).unwrap();
        assert_eq!(actual_masses, expected_masses);
        let result = frame.certify_gram_frame_unrounded_masses(&gram, &actual_masses).unwrap();
        let scale_squared = format.scale().pow(2);
        let denominator = &scale_squared * &scale_squared * 4u8;
        let total: BigInt = expected_masses.iter().cloned().map(BigInt::from).sum();
        assert_eq!(result.denominator, denominator);
        for entry in 0..16 {
            let (row,col)=(entry/4,entry%4); let mut re=BigInt::zero(); let mut im=BigInt::zero();
            for (ray,mass) in frame.rays.iter().zip(&expected_masses) {
                let mass=BigInt::from(mass.clone());
                re += &mass*(&ray[row].re*&ray[col].re+&ray[row].im*&ray[col].im);
                im += mass*(&ray[row].re*&ray[col].im-&ray[row].im*&ray[col].re);
            }
            re *= 5u8; im *= 5u8;
            if row==col { re -= &total*&scale_squared; }
            let round=|v:&BigInt| {
                let mag=(v.abs()+&denominator/2u8)/&denominator;
                if v.is_negative() { -mag } else { mag }
            };
            assert_eq!(result.rounded.recovered[entry], (round(&re),round(&im)));
            assert_eq!(result.signed_residual_numerators[entry],
                (re-&gram[entry].0*&denominator, im-&gram[entry].1*&denominator));
        }
        let mut changed=gram.clone();
        for col in 1..4 {
            let (re,im)=&gram[col]; changed[col]=(-im,re.clone()); changed[4*col]=(-im,-re);
        }
        let changed_masses=frame.gram_mass_numerators(&changed).unwrap();
        frame.certify_gram_frame_unrounded_masses(&changed,&changed_masses).unwrap();
        assert!(frame.certify_gram_frame_unrounded_masses(&changed,&actual_masses).is_err());
    }
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
    fn sic_phase_likelihoods_match_the_four_ideal_phase_poles() {
        let source = BigUint::from(1u8) << 128usize;
        let algebra = FibonacciPair::new(&source).unwrap();
        let sic = FixedQuquartSic::new(algebra.format()).unwrap();
        for pole in 0..4usize {
            let masses = sic
                .phase_outcome_masses(
                    &BigUint::from(pole),
                    &BigUint::from(4u8),
                    &BigUint::one(),
                    &BigUint::zero(),
                    &BigUint::one(),
                )
                .unwrap();
            let total: BigUint = masses.iter().cloned().sum();
            let expected_total = algebra.format().scale().to_biguint().unwrap().pow(2) * 4u8;
            assert!(
                (BigInt::from(total) - BigInt::from(expected_total)).abs()
                    < algebra.format().scale() * 4096u16
            );
            let channel = pole;
            for (outcome, ray) in sic.rays().iter().enumerate() {
                let expected = mass(&ray[channel]);
                assert_eq!(masses[outcome], expected);
            }
        }
        let mut evidence = SicPhaseEvidence::new(algebra.format()).unwrap();
        let wider_source = BigUint::one() << 256usize;
        assert!(evidence.validate_source(&wider_source).is_err());
        evidence.validate_source(&source).unwrap();
        for pole in 0..4usize {
            evidence
                .add_hypothesis(BigUint::from(pole), BigUint::from(4u8))
                .unwrap();
        }
        assert!(evidence
            .add_hypothesis(BigUint::from(2u8), BigUint::from(8u8))
            .is_err());
        let outcome = crate::sic::SixteenOutcome::new(5).unwrap();
        assert!(evidence
            .observe(
                QuquartSicOutcome::OutsideCarrier,
                &BigUint::one(),
                &BigUint::zero(),
                &BigUint::one(),
            )
            .is_err());
        evidence
            .observe(
                QuquartSicOutcome::Carrier(outcome),
                &BigUint::one(),
                &BigUint::zero(),
                &BigUint::one(),
            )
            .unwrap();
        for (numerator, denominator, score) in evidence.scores() {
            let likelihoods = sic
                .phase_outcome_masses(
                    numerator,
                    denominator,
                    &BigUint::one(),
                    &BigUint::zero(),
                    &BigUint::one(),
                )
                .unwrap();
            assert_eq!(*score, likelihoods[outcome.mask() as usize]);
        }
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
            for pole in 0..4usize {
                let likelihoods = sic
                    .phase_outcome_masses(
                        &BigUint::from(pole),
                        &BigUint::from(4u8),
                        &BigUint::one(),
                        &BigUint::zero(),
                        &BigUint::one(),
                    )
                    .unwrap();
                for (outcome, ray) in sic.rays().iter().enumerate() {
                    assert_eq!(likelihoods[outcome], mass(&ray[pole]));
                }
            }
            for digit in [
                QuquartDigit::T,
                QuquartDigit::F,
                QuquartDigit::InfoT,
                QuquartDigit::InfoF,
            ] {
                let mut state = QuquartCarrier::basis(&algebra, digit);
                let before = state.amplitudes.clone();
                state.fourier_target(false);
                for &channel in &COMPUTATIONAL_CHANNELS {
                    assert_eq!(mass(&state.amplitudes[channel]), &square / 4u8);
                }
                state.fourier_target(true);
                for (a, b) in state.amplitudes.iter().zip(before.iter()) {
                    assert_eq!(a.re, b.re);
                    assert_eq!(a.im, b.im);
                }
            }
            let mut rotated = QuquartCarrier::basis(&algebra, QuquartDigit::T);
            rotated.fourier_target(false);
            rotated.feedback_target(&BigUint::one(), 1).unwrap();
            rotated.fourier_target(true);
            let peak = COMPUTATIONAL_CHANNELS[3];
            assert!(
                (BigInt::from(mass(&rotated.amplitudes[peak])) - BigInt::from(square.clone()))
                    .abs()
                    < &scale * 256u16
            );
            for &channel in &COMPUTATIONAL_CHANNELS[..3] {
                assert!(mass(&rotated.amplitudes[channel]) < BigUint::from(4096u16));
            }
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
