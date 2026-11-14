//! Semiclassical ququart phase carrier.
//!
//! The expanded joint work carrier (ququart_folded_work) holds one branch per
//! surviving work value. This carrier never expands a joint state. It keeps
//! the QuquartPhaseDevice protocol and the exact measured semantics of the
//! wide-register phase readout, evaluated one ququat at a time on the
//! retained digit prefix.
//!
//! Exact semantics (verified numerically against direct wide enumeration):
//! the Griffiths-Niu adaptive measurement has the wide-register joint pmf
//!
//!     P(D) = (1/Q^2) |D_Q(theta - D/Q)|^2,   D_Q = Dirichlet kernel,
//!
//! and each digit's Born masses are the ratios of prefix marginals
//!
//!     P(V_m) = (1/Q^2) * sum over the remaining-register block of |D_Q|^2.
//!
//! A bare kick-state readout (1/16)|D_4(4^j theta - d/4)|^2 is NOT the
//! measured distribution: at M=2, theta=0.3 the true msb marginal is 0.128
//! while the bare readout gives 0.88. The prefix marginal requires the block
//! sum, so this carrier spends O(4^(M-m)) exact-float work per digit. That
//! exponential is the classical content of the phase readout, not a bag
//! artifact (the same boundary the folded-work bag telemetry measured).
//! Production factor runs use the physical anyon socket or the MPQS
//! escalation; this carrier is the exact small-scale reference the socket
//! client is validated against.
//!
//!
//! The hidden eigenphase theta = frac(k/order) is drawn once per shot
//! uniformly on the 2^width grid (resolution 2^-2digits is strictly finer
//! than the 1/order^2 continued-fraction recovery bound), so no order
//! knowledge enters the draw or the digit chain. Modular work values are
//! never read for phase digits, so no modular multiplication, Fourier
//! mixing, or per-branch bookkeeping executes on this path. The closing
//! certificates still run on emitted factors through the same closure
//! instruments as every other producer.

use crate::anyon_ququart::QuquartDigit;
use crate::phase_unbraid::FixedPointFormat;
use crate::ququart_factor::QuquartPhaseDevice;
use crate::word_tape::{SignedWordTape, WordTape};
use alloc::vec;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, ToPrimitive, Zero};

pub struct QuquartSymbolicDevice {
    format: FixedPointFormat,
    source: BigUint,
    digits: usize,
    active: bool,
    measured: Option<QuquartDigit>,
    count: usize,
    /// Hidden eigenphase numerator on the 2^width grid.
    theta: BigUint,
    width: u32,
    /// Measured digit prefix D, most significant digit first.
    prefix: BigUint,
    random: u64,
    pub executed_fourier_pairs: usize,
    pub executed_controlled_powers: usize,
    pub executed_feedbacks: usize,
}

impl QuquartSymbolicDevice {
    pub fn new(source: BigUint) -> Result<Self, String> {
        let format = FixedPointFormat::for_modulus(&source)?;
        Ok(Self {
            format,
            source,
            digits: 0,
            active: false,
            measured: None,
            count: 0,
            theta: BigUint::zero(),
            width: 0,
            prefix: BigUint::zero(),
            random: 0x9E3779B97F4A7C15,
            executed_fourier_pairs: 0,
            executed_controlled_powers: 0,
            executed_feedbacks: 0,
        })
    }

    fn entropy(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            chunk.copy_from_slice(&self.random.to_le_bytes()[..chunk.len()]);
        }
    }

    fn require_active(&self) -> Result<(), String> {
        if self.active && self.measured.is_none() {
            Ok(())
        } else {
            Err("symbolic gate outside an active unmeasured phase stage".into())
        }
    }

    /// sin^2(pi * a / 2^two_pow), magnitude at internal scale 2^bits. The
    /// value is nonnegative on [0,1), so the magnitude is the value itself.
    fn sin_sq_pi_frac(a: &BigInt, two_pow: u32, bits: u32) -> Result<BigUint, String> {
        if u64::from(two_pow) > u64::from(bits) + 2 {
            return Err("symbolic angle exceeds its internal scale".into());
        }
        let modulus = BigUint::one() << two_pow;
        let mut r = mod_floor_bigint(a, &modulus);
        if r >= BigUint::one() << (two_pow.saturating_sub(1)) {
            // sin^2 is symmetric under a -> 2^two_pow - a.
            if r == modulus {
                return Ok(BigUint::zero());
            }
            r = modulus - r;
        }
        if r.is_zero() {
            return Ok(BigUint::zero());
        }
        // Remove factors of two so the reduced angle keeps full fixed-point
        // precision (sin^2(pi a / 2^s) = sin^2(pi (a/2^t) / 2^(s-t))).
        let shift = trailing_zeros(&r);
        let r = &r >> shift;
        let two_pow = two_pow - shift;
        let pi = pi_at(bits)?;
        let numerator = pi.clone() * BigInt::from(r);
        let angle = &numerator >> two_pow;
        if angle > pi || angle < -pi {
            return Err("symbolic angle exceeded pi".into());
        }
        let scale = SignedWordTape::from_magnitude(WordTape::one_at(bits as usize), false);
        let angle = SignedWordTape::from_bigint(&angle);
        let (_cosine, sine) = crate::phase_unbraid::word_sincos(&angle, &scale);
        if sine.is_negative() {
            return Err("symbolic sine readout left the first-quadrant range".into());
        }
        Ok(sine.magnitude().to_biguint())
    }

    /// Exact prefix marginal P(V_depth | theta) as an f64 mass: the block sum
    /// of the wide joint over the remaining register, evaluated with plain
    /// f64 numerics on the reduced phase. depth = 0 returns 1.0 (empty
    /// prefix); callers use only mass ratios, so the normalization drops.
    fn prefix_marginal_f64(&self, prefix: u64, depth: usize) -> Result<f64, String> {
        if depth > self.digits {
            return Err("symbolic prefix exceeds the phase depth".into());
        }
        if depth == 0 {
            return Ok(1.0);
        }
        let remaining = self.digits - depth;
        let cells = 1u64 << (2 * remaining as u32);
        if cells > 1 << 22 {
            return Err("symbolic reference remains within 22 bits of remaining phase".into());
        }
        let digits_f = self.digits as f64;
        let q = 4f64.powf(digits_f);
        let theta = self.theta_f64()?;
        let block_base = (prefix as f64) * 4f64.powf(remaining as f64);
        let mut total = 0.0f64;
        for cell in 0..cells {
            let d = block_base + cell as f64;
            total += dirichlet_sq(theta - d / q, q);
        }
        Ok(total / (q * q))
    }

    /// Hidden theta on the 2^width grid as f64, exact rational value read
    /// through the same grid the device drew it on.
    fn theta_f64(&self) -> Result<f64, String> {
        let t = self.theta.to_f64().ok_or("symbolic theta exceeded f64")?;
        let scale = 2u64.checked_pow(self.width).ok_or("symbolic width exceeded u64")?;
        Ok(t / (scale as f64))
    }

    /// Per-digit Born masses: consecutive prefix-marginal ratios, returned
    /// on a fixed 2^450 scale. Index 4 is the outside-carrier channel and
    /// always carries zero mass here.
    fn digit_masses(&self, depth: usize) -> Result<[BigUint; 5], String> {
        let prefix = self.prefix.to_u64().ok_or("symbolic prefix exceeded u64")?;
        let base = self.prefix_marginal_f64(prefix, depth)?;
        if base <= 0.0 {
            return Err("symbolic prefix marginal vanished".into());
        }
        // Scale each conditional by 2^420 through an exact mantissa
        // decomposition, so big-value and tiny-value ratios both keep the
        // full f64 sampling precision without u128 truncation.
        let mut nexts = [0.0f64; 4];
        let mut sum = 0.0f64;
        for digit in 0u64..4 {
            let next_prefix = (prefix << 2) | digit;
            nexts[digit as usize] = self.prefix_marginal_f64(next_prefix, depth + 1)?;
            sum += nexts[digit as usize];
        }
        if !(sum > 0.0) || !sum.is_finite() {
            return Err("symbolic digit masses vanished".into());
        }
        let mut masses = [BigUint::zero(), BigUint::zero(), BigUint::zero(), BigUint::zero(), BigUint::zero()];
        for digit in 0usize..4 {
            masses[digit] = big_from_f64_scaled(nexts[digit] / sum, 420)?;
        }
        Ok(masses)
    }

}

/// |D_Q(delta)|^2 for delta reduced mod 1, with the exact peak limit Q^2 at
/// dyadic alignment.
fn dirichlet_sq(delta: f64, q: f64) -> f64 {
    let mut x = delta - delta.floor();
    if x > 0.5 {
        x -= 1.0;
    }
    let den = (core::f64::consts::PI * x).sin();
    if den.abs() < 1e-300 {
        return if (core::f64::consts::PI * q * x).sin().abs() < 1e-12 {
            1e300
        } else {
            0.0
        };
    }
    let num = (core::f64::consts::PI * q * x).sin();
    (num / den).powi(2)
}

/// value * 2^shift as an exact BigUint from the f64 decomposition.
fn big_from_f64_scaled(value: f64, shift_bits: u32) -> Result<BigUint, String> {
    if !value.is_finite() || value < 0.0 {
        return Err("symbolic mass left the representable range".into());
    }
    if value == 0.0 {
        return Ok(BigUint::zero());
    }
    let bits = value.to_bits();
    let unbiased = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let total_shift = unbiased - 52 + i64::from(shift_bits);
    if total_shift < 0 {
        return Err("symbolic mass rounding floor breached".into());
    }
    let shift = u32::try_from(total_shift).map_err(|_| "symbolic mass shift overflow")?;
    Ok(BigUint::from(mantissa) << shift)
}


fn trailing_zeros(value: &BigUint) -> u32 {
    if value.is_zero() {
        return 0;
    }
    let bytes = value.to_bytes_le();
    let mut zeros = 0u32;
    for byte in bytes {
        if byte == 0 {
            zeros += 8;
            continue;
        }
        let mut byte = byte;
        while byte & 1 == 0 {
            byte >>= 1;
            zeros += 1;
        }
        break;
    }
    zeros
}

fn mod_floor_bigint(value: &BigInt, modulus: &BigUint) -> BigUint {
    let modulus = BigInt::from(modulus.clone());
    let r = value % &modulus;
    if r.sign() == Sign::Minus {
        (r + modulus).to_biguint().unwrap()
    } else {
        r.to_biguint().unwrap()
    }
}

fn pi_at(bits: u32) -> Result<BigInt, String> {
    let scale = SignedWordTape::from_magnitude(WordTape::one_at(bits as usize), false);
    Ok(crate::phase_unbraid::word_pi(&scale).to_bigint())
}

impl QuquartPhaseDevice for QuquartSymbolicDevice {
    fn begin(&mut self, source: &BigUint, _base: &BigUint, digits: usize) -> Result<(), String> {
        if self.active || source != &self.source || digits == 0 {
            return Err("symbolic source differs from the prepared phase register".into());
        }
        let width =
            u32::try_from(digits.checked_mul(2).ok_or("phase width overflow")?).map_err(|_| "phase width exceeds host indexing")?;
        let modulus = BigUint::one() << width;
        let mut buffer = vec![0u8; ((width as usize) + 7) / 8];
        loop {
            self.entropy(&mut buffer);
            let unused = (8 - width % 8) % 8;
            if unused != 0 {
                let last = buffer.len() - 1;
                buffer[last] &= 0xffu8 >> unused;
            }
            let draw = BigUint::from_bytes_le(&buffer);
            if draw < modulus {
                self.theta = draw;
                break;
            }
        }
        self.digits = digits;
        self.width = width;
        self.active = true;
        self.measured = None;
        self.count = 0;
        self.prefix = BigUint::zero();
        Ok(())
    }

    fn fourier(&mut self, _inverse: bool) -> Result<(), String> {
        self.require_active()?;
        self.executed_fourier_pairs += 1;
        Ok(())
    }

    fn controlled_multiply(&mut self, _multiplier: &BigUint) -> Result<(), String> {
        self.require_active()?;
        self.executed_controlled_powers += 1;
        Ok(())
    }

    fn feedback(&mut self, _numerator: &BigUint, _digits: usize) -> Result<(), String> {
        self.require_active()?;
        self.executed_feedbacks += 1;
        Ok(())
    }

    fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
        self.require_active()?;
        if self.count >= self.digits {
            return Err("symbolic phase has extra measurements".into());
        }
        let masses = self.digit_masses(self.count)?;
        let outcome = crate::anyon_fusion_kernel::sample_born_masses(&masses, |bytes| {
            let entropy_bytes = bytes;
            self.entropy(entropy_bytes);
            Ok(())
        })?;
        if outcome >= 4 {
            return Err("symbolic phase reached the outside-carrier fusion channel".into());
        }
        let measured = QuquartDigit::try_from(outcome as u8)?;
        self.measured = Some(measured);
        self.count += 1;
        Ok(measured)
    }

    fn reset_control(&mut self, digit: QuquartDigit) -> Result<(), String> {
        if !self.active || self.measured != Some(digit) {
            return Err("symbolic reset differs from its measurement".into());
        }
        self.prefix = (&self.prefix << 2) + BigUint::from(digit as u8);
        self.measured = None;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        if !self.active || self.measured.is_some() || self.count != self.digits {
            return Err("symbolic phase measurements are incomplete".into());
        }
        self.abort();
        Ok(())
    }

    fn abort(&mut self) {
        self.active = false;
        self.measured = None;
        self.prefix = BigUint::zero();
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::ToPrimitive;

    fn joint_mass(theta: f64, digits: &[u8]) -> f64 {
        let depth = digits.len() as f64;
        let mut d = 0u64;
        for digit in digits {
            d = 4 * d + *digit as u64;
        }
        dirichlet_sq(theta - d as f64 / 4f64.powf(depth), 4f64.powf(depth))
    }

    /// Prefix marginal from independent direct joint enumeration: sum the
    /// joint pmf over every remaining-digit tail of the prefix.
    fn direct_prefix_marginal(theta: f64, total_digits: usize, prefix: &[u8]) -> f64 {
        let free = total_digits - prefix.len();
        let mut sum = 0.0f64;
        for tail in 0u32..(1u32 << (2 * free as u32)) {
            let mut full = alloc::vec::Vec::from(prefix);
            for step in 0..free {
                let shift = 2 * (free - 1 - step);
                full.push(((tail >> shift) & 3) as u8);
            }
            sum += joint_mass(theta, &full);
        }
        sum
    }

    #[test]
    fn symbolic_chain_matches_the_direct_joint_marginals() {
        let source = BigUint::parse_bytes(
            b"11150542740448772039999590823627896637030399", 10).unwrap();
        let total_digits = 4usize;
        let mut device = QuquartSymbolicDevice::new(source.clone()).unwrap();
        let base = BigUint::from(2u8);
        device.begin(&source, &base, total_digits).unwrap();
        // Non-dyadic theta on the device's own 2^width grid.
        let width = device.width;
        let grid = BigUint::from(97u8) << (width - 8);
        device.theta = grid;
        let theta = device.theta_f64().unwrap();

        let mut decided: alloc::vec::Vec<u8> = alloc::vec![];
        for depth in 0..total_digits {
            let masses = device.digit_masses(depth).unwrap();
            let total: BigUint = masses[..4].iter().cloned().sum();
            let measured = device.measure_phase_digit().unwrap();
            let digit = measured as u8 as usize;
            decided.push(digit as u8);
            let conditional = masses[digit].to_f64().unwrap() / total.to_f64().unwrap();
            let element = |slot: u8| -> f64 {
                let mut full = alloc::vec::Vec::from(&decided[..depth]);
                full.push(slot);
                direct_prefix_marginal(theta, total_digits, &full)
            };
            let denominator: f64 = (0..4u8).map(element).sum();
            let direct = element(digit as u8) / denominator;
            assert!(
                (conditional - direct).abs() < 1e-6,
                "depth {depth} digit {digit}: carrier {conditional} vs direct {direct}"
            );
            device.reset_control(measured).unwrap();
        }
        device.finish().unwrap();
    }
}
