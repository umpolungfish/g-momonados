//! Joint ququart/work amplitudes for source-bound phase measurement.
//! The physical Fourier operator is contracted from the prepared Fibonacci
//! braid. Modular powers act on the retained work register; no period is
//! supplied to the Born measurement.
use crate::anyon_fusion_kernel::sample_born_masses;
use crate::anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::anyon_ququart::QuquartDigit;
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::ququart_factor::QuquartPhaseDevice;
use alloc::{collections::BTreeMap, string::String};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

fn zero() -> FixedComplex { FixedComplex { re: BigInt::zero(), im: BigInt::zero() } }
fn mass(z: &FixedComplex) -> BigUint {
    (&z.re * &z.re + &z.im * &z.im).to_biguint().unwrap()
}
fn add(to: &mut FixedComplex, from: FixedComplex) { to.re += from.re; to.im += from.im; }

pub struct QuquartWorkDevice {
    source: BigUint,
    format: FixedPointFormat,
    fourier: PairMatrix,
    state: BTreeMap<BigUint, [FixedComplex; 5]>,
    random: u64,
    active: bool,
    measured: Option<QuquartDigit>,
    expected_digits: usize,
    measured_digits: usize,
    pub peak_support: usize,
}

impl QuquartWorkDevice {
    pub fn new(source: BigUint, fourier: PairMatrix, seed: u64) -> Result<Self, String> {
        let format = FixedPointFormat::for_modulus(&source)?;
        Ok(Self { source, format, fourier, state: BTreeMap::new(),
            random: seed.max(1), active: false, measured: None,
            expected_digits: 0, measured_digits: 0, peak_support: 0 })
    }

    fn entropy(random: &mut u64, bytes: &mut [u8]) -> Result<(), String> {
        for chunk in bytes.chunks_mut(8) {
            *random ^= *random << 13;
            *random ^= *random >> 7;
            *random ^= *random << 17;
            chunk.copy_from_slice(&random.to_le_bytes()[..chunk.len()]);
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), String> {
        if self.active && self.measured.is_none() { Ok(()) }
        else { Err("ququart gate outside an unmeasured active stage".into()) }
    }

    fn apply_fourier(&mut self, inverse: bool) -> Result<(), String> {
        self.require_active()?;
        let matrix = if inverse { self.fourier.adjoint() } else { self.fourier.clone() };
        for ray in self.state.values_mut() {
            let old = ray.clone();
            *ray = core::array::from_fn(|row| {
                let mut value = zero();
                for (col, amplitude) in old.iter().enumerate() {
                    add(&mut value, matrix.0[5 * row + col].mul(amplitude, &self.format));
                }
                value
            });
        }
        Ok(())
    }
}

impl QuquartPhaseDevice for QuquartWorkDevice {
    fn begin(&mut self, source: &BigUint, _base: &BigUint, digits: usize) -> Result<(), String> {
        if self.active || source != &self.source || digits == 0 {
            return Err("ququart preparation differs from its compiled source".into());
        }
        // A fresh uniformly sampled residue is one member of the uniform
        // mixed ensemble. Its coherent work state persists through the shot.
        let register = crate::anyon_fusion_kernel::PreparedRegister::uniform(source,
            |bytes| Self::entropy(&mut self.random, bytes))?;
        let mut ray = core::array::from_fn(|_| zero());
        ray[COMPUTATIONAL_CHANNELS[0]].re = self.format.scale();
        self.state = BTreeMap::from([(register.residue().clone(), ray)]);
        self.active = true;
        self.measured = None;
        self.expected_digits = digits;
        self.measured_digits = 0;
        self.peak_support = self.peak_support.max(1);
        Ok(())
    }

    fn fourier(&mut self, inverse: bool) -> Result<(), String> { self.apply_fourier(inverse) }

    fn controlled_multiply(&mut self, multiplier: &BigUint) -> Result<(), String> {
        self.require_active()?;
        let mut a = multiplier.clone();
        let mut b = self.source.clone();
        while !b.is_zero() { let r = &a % &b; a = b; b = r; }
        if !a.is_one() { return Err("controlled work operator must be a permutation".into()); }
        let mut next: BTreeMap<BigUint, [FixedComplex; 5]> = BTreeMap::new();
        for (residue, ray) in &self.state {
            let mut image = residue.clone();
            for &channel in &COMPUTATIONAL_CHANNELS {
                let amplitude = &ray[channel];
                if !amplitude.re.is_zero() || !amplitude.im.is_zero() {
                    let output = next.entry(image.clone()).or_insert_with(|| core::array::from_fn(|_| zero()));
                    add(&mut output[channel], amplitude.clone());
                }
                image = image * multiplier % &self.source;
            }
            let leaked = &ray[LEAKAGE_CHANNEL];
            if !leaked.re.is_zero() || !leaked.im.is_zero() {
                let output = next.entry(residue.clone()).or_insert_with(|| core::array::from_fn(|_| zero()));
                add(&mut output[LEAKAGE_CHANNEL], leaked.clone());
            }
        }
        self.peak_support = self.peak_support.max(next.len());
        self.state = next;
        Ok(())
    }

    fn feedback(&mut self, numerator: &BigUint, digits: usize) -> Result<(), String> {
        self.require_active()?;
        let bits = digits.checked_mul(2).ok_or("feedback width overflow")?;
        let denominator = BigUint::one() << bits;
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let phase = FixedComplex::winding_twiddle(
                &-BigInt::from(numerator * digit), &denominator, &self.format)?;
            for ray in self.state.values_mut() { ray[channel] = ray[channel].mul(&phase, &self.format); }
        }
        Ok(())
    }

    fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
        self.require_active()?;
        if self.measured_digits >= self.expected_digits { return Err("too many ququart measurements".into()); }
        let mut masses: [BigUint; 5] = core::array::from_fn(|_| BigUint::zero());
        for ray in self.state.values() {
            for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() { masses[digit] += mass(&ray[channel]); }
            masses[4] += mass(&ray[LEAKAGE_CHANNEL]);
        }
        let outcome = sample_born_masses(&masses, |bytes| Self::entropy(&mut self.random, bytes))?;
        if outcome == 4 { return Err("ququart measurement reached the outside-carrier fusion channel".into()); }
        let digit = QuquartDigit::try_from(outcome as u8)?;
        let selected = COMPUTATIONAL_CHANNELS[outcome];
        let norm = BigInt::from(masses[outcome].sqrt());
        if norm.is_zero() { return Err("ququart selected a zero Born mass".into()); }
        let scale = self.format.scale();
        for ray in self.state.values_mut() {
            let amplitude = FixedComplex { re: &ray[selected].re * &scale / &norm,
                im: &ray[selected].im * &scale / &norm };
            *ray = core::array::from_fn(|_| zero());
            ray[selected] = amplitude;
        }
        self.state.retain(|_, ray| !ray[selected].re.is_zero() || !ray[selected].im.is_zero());
        self.measured = Some(digit);
        self.measured_digits += 1;
        Ok(digit)
    }

    fn reset_control(&mut self, digit: QuquartDigit) -> Result<(), String> {
        if !self.active || self.measured != Some(digit) { return Err("reset differs from measured pole".into()); }
        let selected = COMPUTATIONAL_CHANNELS[digit as usize];
        for ray in self.state.values_mut() {
            let value = ray[selected].clone();
            *ray = core::array::from_fn(|_| zero());
            ray[COMPUTATIONAL_CHANNELS[0]] = value;
        }
        self.measured = None;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        if !self.active || self.measured.is_some() || self.measured_digits != self.expected_digits {
            return Err("ququart shot has incomplete phase measurements".into());
        }
        self.state.clear(); self.active = false; Ok(())
    }
    fn abort(&mut self) { self.state.clear(); self.active = false; self.measured = None; }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ideal_fourier(n: &BigUint) -> PairMatrix {
        let format = FixedPointFormat::for_modulus(n).unwrap();
        let mut matrix = PairMatrix(core::array::from_fn(|_| zero()));
        for (row, &r) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (col, &c) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let mut z = FixedComplex::winding_twiddle(&BigInt::from(row * col), &BigUint::from(4u8), &format).unwrap();
                z.re /= 2u8; z.im /= 2u8;
                matrix.0[5*r+c] = z;
            }
        }
        matrix.0[5*LEAKAGE_CHANNEL+LEAKAGE_CHANNEL].re = format.scale();
        matrix
    }

    #[test]
    fn nonpermutation_failure_preserves_the_original_coherent_state() {
        let n = BigUint::parse_bytes(b"74190557381655886253556359637698917958655348096397072330011641798610025580603", 10).unwrap();
        let mut device = QuquartWorkDevice::new(n.clone(), ideal_fourier(&n), 1729).unwrap();
        device.begin(&n, &BigUint::from(2u8), 260).unwrap();
        device.fourier(false).unwrap();
        let before: Vec<_> = device.state.iter().map(|(r,a)| (r.clone(), a.iter().map(|z| (z.re.clone(),z.im.clone())).collect::<Vec<_>>())).collect();
        assert!(device.controlled_multiply(&n).is_err());
        let after: Vec<_> = device.state.iter().map(|(r,a)| (r.clone(), a.iter().map(|z| (z.re.clone(),z.im.clone())).collect::<Vec<_>>())).collect();
        assert_eq!(before, after);
        device.abort();
        assert!(device.state.is_empty());
    }
}
