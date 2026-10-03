//! phase_unbraid.rs — the phase-based unbraid: factors from a phase
//! readout, not a search.
//!
//! BigUint lift, no static caps. q is dynamic, derived from bits(N):
//!   q = 2 * bits(N) + 8, M = 2^q.  All arithmetic on BigUint. The
//!   phase register is measured one control bit at a time, while the
//!   sparse complex residue register survives. CUDA applies fixed-point
//!   feedback and Hadamard butterflies. BigUint addresses both registers.
//!
//! Dense CPU reference pipeline:
//!   1. CPU reference constructs a collapsed comb using a known period
//!   2. exact QFT in-place radix-2 FFT, O(M log M), streamed in blocks
//!   3. one Born measurement, seeded xorshift shot
//!   4. k/M winding → continued-fraction convergents → reduced s/r0
//!   5. period lift by BigUint powm, gcd(a^(r/2) ± 1, N) — one gcd
//!   6. factors emitted AS WORDS through native_numeral (D(p), D(q), Γ
//!      carrier) and verified word-natively (multiply_via_word +
//!      syzygy_preserves).
//!
//! The recycled executor prepares |1>, applies controlled modular powers,
//! and measures all q phase bits with feedback. Its period comes from those
//! measurements. The dense CPU reference uses a period-informed comb.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::native_numeral;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Signed, Zero};

#[derive(Clone, Copy, Debug)]
pub struct Cx { pub re: f64, pub im: f64 }

impl Cx {
    fn zero() -> Self { Cx { re: 0.0, im: 0.0 } }
    fn new(re: f64, im: f64) -> Self { Cx { re, im } }
    fn scale(self, s: f64) -> Self { Cx { re: self.re * s, im: self.im * s } }
    fn norm2(self) -> f64 { self.re * self.re + self.im * self.im }
}
impl core::ops::Add for Cx {
    type Output = Cx;
    fn add(self, o: Cx) -> Cx { Cx { re: self.re + o.re, im: self.im + o.im } }
}
impl core::ops::Sub for Cx {
    type Output = Cx;
    fn sub(self, o: Cx) -> Cx { Cx { re: self.re - o.re, im: self.im - o.im } }
}
impl core::ops::Mul for Cx {
    type Output = Cx;
    fn mul(self, o: Cx) -> Cx {
        Cx { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedPointFormat {
    pub modulus_bits: u64,
    pub w_bits: u64,
}

impl FixedPointFormat {
    pub fn for_modulus(n: &BigUint) -> Result<Self, String> {
        Self::for_modulus_bits(n.bits().saturating_sub(1))
    }

    pub fn for_modulus_bits(modulus_bits: u64) -> Result<Self, String> {
        let w_bits = modulus_bits.checked_mul(2).and_then(|bits| bits.checked_add(16))
            .ok_or("fixed-point W overflow for modulus width")?;
        usize::try_from(w_bits).map_err(|_| "fixed-point W exceeds host limb indexing")?;
        Ok(Self { modulus_bits, w_bits })
    }

    pub fn for_phase_register(qubits: u64) -> Result<Self, String> {
        let modulus_bits = qubits.saturating_sub(10) / 2;
        Self::for_modulus_bits(modulus_bits)
    }

    pub fn scale(&self) -> BigInt { BigInt::one() << self.w_bits as usize }

    pub fn encode_f64(&self, value: f64) -> Result<BigInt, String> {
        if !value.is_finite() { return Err("cannot encode a non-finite fixed-point value".into()); }
        let bits = value.to_bits();
        let negative = bits >> 63 != 0;
        let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1u64 << 52) - 1);
        if exponent_bits == 0 && fraction == 0 { return Ok(BigInt::zero()); }
        let (mantissa, exponent) = if exponent_bits == 0 {
            (fraction, -1022 - 52)
        } else {
            ((1u64 << 52) | fraction, exponent_bits - 1023 - 52)
        };
        let shift = self.w_bits as i128 + i128::from(exponent);
        let magnitude = if shift >= 0 {
            BigInt::from(mantissa) << usize::try_from(shift).map_err(|_| "fixed-point encode shift overflow")?
        } else {
            let right = usize::try_from(-shift).map_err(|_| "fixed-point encode shift overflow")?;
            (BigInt::from(mantissa) + (BigInt::one() << right.saturating_sub(1))) >> right
        };
        Ok(if negative { -magnitude } else { magnitude })
    }

    pub fn decode_f64(&self, value: &BigInt) -> f64 {
        if value.is_zero() { return 0.0; }
        let negative = value.sign() == Sign::Minus;
        let magnitude = value.abs().to_biguint().unwrap_or_else(BigUint::zero);
        let bit_count = magnitude.bits();
        let shift = bit_count.saturating_sub(53);
        let top = (&magnitude >> usize::try_from(shift).unwrap_or(usize::MAX)).to_u64_digits().first().copied().unwrap_or(0);
        let exponent = i128::from(shift) - i128::from(self.w_bits);
        let decoded = if exponent > 1023 { f64::INFINITY }
            else if exponent < -1074 { 0.0 }
            else { (top as f64) * 2.0f64.powi(exponent as i32) };
        if negative { -decoded } else { decoded }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedComplex {
    pub re: BigInt,
    pub im: BigInt,
}

impl FixedComplex {
    /// A rational winding, exp(2 pi i numerator / denominator), evaluated at
    /// the source-derived fixed precision. Reduce before trigonometry so even
    /// an arbitrarily wide numerator cannot produce an unbounded Taylor angle.
    pub fn winding_twiddle(numerator: &BigInt, denominator: &BigUint, format: &FixedPointFormat) -> Result<Self, String> {
        if denominator.is_zero() { return Err("winding denominator is zero".into()); }
        let denominator = BigInt::from(denominator.clone());
        let mut reduced = ((numerator % &denominator) + &denominator) % &denominator;
        if &reduced * 2 > denominator { reduced -= &denominator; }
        let work_bits = format.w_bits.checked_add(32).ok_or("winding precision overflow")?;
        let shift = usize::try_from(work_bits).map_err(|_| "winding precision exceeds host indexing")?;
        let scale = BigInt::one() << shift;
        let angle = fixed_round_div(fixed_pi(&scale) * 2 * reduced, &denominator);
        let (re, im) = fixed_sincos(&angle, &scale);
        let rounding = BigInt::one() << 31usize;
        let down = |value: BigInt| if value.is_negative() { -((-value + &rounding) >> 32usize) } else { (value + &rounding) >> 32usize };
        Ok(Self { re: down(re), im: down(im) })
    }

    pub fn qft_twiddle(index: &BigUint, stage: u64, inverse: bool, format: &FixedPointFormat) -> Result<Self, String> {
        Self::qft_twiddle_table(core::slice::from_ref(index), stage, inverse, format).map(|mut values| values.remove(0))
    }

    pub fn qft_twiddle_table(indices: &[BigUint], stage: u64, inverse: bool, format: &FixedPointFormat) -> Result<Vec<Self>, String> {
        let work_bits = format.w_bits.checked_add(32).ok_or("fixed-point trigonometry precision overflow")?;
        let work_shift = usize::try_from(work_bits).map_err(|_| "fixed-point trigonometry precision exceeds host indexing")?;
        let output_shift = usize::try_from(work_bits - format.w_bits).map_err(|_| "fixed-point guard precision exceeds host indexing")?;
        let scale = BigInt::one() << work_shift;
        let pi = fixed_pi(&scale);
        let stage_shift = usize::try_from(stage).map_err(|_| "QFT stage exceeds fixed-point phase indexing")?;
        let round = BigInt::one() << output_shift.saturating_sub(1);
        let round_down = |value: BigInt| if value.is_negative() { -((-value + &round) >> output_shift) } else { (value + &round) >> output_shift };
        indices.iter().map(|index| {
            let mut angle = (&pi * BigInt::from(index.clone())) >> stage_shift;
            if !inverse { angle = -angle; }
            let (cosine, sine) = fixed_sincos(&angle, &scale);
            Ok(Self { re: round_down(cosine), im: round_down(sine) })
        }).collect()
    }

    pub fn from_f64(re: f64, im: f64, format: &FixedPointFormat) -> Result<Self, String> {
        Ok(Self { re: format.encode_f64(re)?, im: format.encode_f64(im)? })
    }

    pub fn decode(&self, format: &FixedPointFormat) -> Cx {
        Cx { re: format.decode_f64(&self.re), im: format.decode_f64(&self.im) }
    }

    pub fn mul(&self, other: &Self, format: &FixedPointFormat) -> Self {
        let shift = format.w_bits as usize;
        let round = BigInt::one() << shift.saturating_sub(1);
        let rounded = |value: BigInt| if value.is_negative() { -((-value + &round) >> shift) } else { (value + &round) >> shift };
        Self {
            re: rounded(&self.re * &other.re - &self.im * &other.im),
            im: rounded(&self.re * &other.im + &self.im * &other.re),
        }
    }

    pub fn half(&self) -> Self {
        let round = BigInt::one();
        let rounded = |value: &BigInt| if value.is_negative() { -((-value + &round) >> 1usize) } else { (value + &round) >> 1usize };
        Self { re: rounded(&self.re), im: rounded(&self.im) }
    }

    pub fn to_bytes(&self, format: &FixedPointFormat) -> Result<Vec<u8>, String> {
        let bytes_per_component = usize::try_from(format.w_bits.saturating_add(2).div_ceil(8))
            .map_err(|_| "fixed-point cell width exceeds host indexing")?;
        let mut bytes = Vec::with_capacity(bytes_per_component.checked_mul(2).ok_or("fixed-point cell width overflow")?);
        for component in [&self.re, &self.im] {
            let mut encoded = component.to_signed_bytes_le();
            if encoded.len() > bytes_per_component { return Err("fixed-point cell exceeds W-bit storage".into()); }
            encoded.resize(bytes_per_component, if component.is_negative() { 0xff } else { 0x00 });
            bytes.extend_from_slice(&encoded);
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8], format: &FixedPointFormat) -> Result<Self, String> {
        let bytes_per_component = usize::try_from(format.w_bits.saturating_add(2).div_ceil(8))
            .map_err(|_| "fixed-point cell width exceeds host indexing")?;
        if bytes.len() != bytes_per_component.checked_mul(2).ok_or("fixed-point cell width overflow")? {
            return Err("fixed-point cell byte length does not match W".into());
        }
        Ok(Self {
            re: BigInt::from_signed_bytes_le(&bytes[..bytes_per_component]),
            im: BigInt::from_signed_bytes_le(&bytes[bytes_per_component..]),
        })
    }
}

fn fixed_round_div(value: BigInt, divisor: &BigInt) -> BigInt {
    let half = divisor >> 1usize;
    if value.is_negative() { -((-value + &half) / divisor) } else { (value + &half) / divisor }
}

fn fixed_arctan_reciprocal(denominator: u32, scale: &BigInt) -> BigInt {
    let divisor = BigInt::from(denominator);
    let square = &divisor * &divisor;
    let mut power = scale / &divisor;
    let mut sum = BigInt::zero();
    let mut index = 0u32;
    while !power.is_zero() {
        let term = &power / BigInt::from(index * 2 + 1);
        if term.is_zero() { break; }
        if index & 1 == 0 { sum += term; } else { sum -= term; }
        power /= &square;
        index += 1;
    }
    sum
}

fn fixed_pi(scale: &BigInt) -> BigInt {
    fixed_arctan_reciprocal(5, scale) * 16 - fixed_arctan_reciprocal(239, scale) * 4
}

fn fixed_sincos(angle: &BigInt, scale: &BigInt) -> (BigInt, BigInt) {
    let angle_squared = fixed_round_div(angle * angle, scale);
    let mut sine_sum = angle.clone();
    let mut sine_term = angle.clone();
    let mut cosine_sum = scale.clone();
    let mut cosine_term = scale.clone();
    let mut index = 1u32;
    loop {
        let sine_divisor = BigInt::from((2 * index) * (2 * index + 1));
        sine_term = fixed_round_div(fixed_round_div(&sine_term * &angle_squared, scale), &sine_divisor);
        let cosine_divisor = BigInt::from((2 * index - 1) * (2 * index));
        cosine_term = fixed_round_div(fixed_round_div(&cosine_term * &angle_squared, scale), &cosine_divisor);
        if sine_term.is_zero() && cosine_term.is_zero() { break; }
        if index & 1 == 1 { sine_sum -= &sine_term; cosine_sum -= &cosine_term; }
        else { sine_sum += &sine_term; cosine_sum += &cosine_term; }
        index += 1;
    }
    (cosine_sum, sine_sum)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseRegisterLayout {
    pub qubits: u64,
    pub tile_amplitudes: u64,
    pub fixed_format: FixedPointFormat,
}

impl DenseRegisterLayout {
    pub fn new(qubits: u64, tile_amplitudes: u64) -> Result<Self, String> {
        if tile_amplitudes == 0 || !tile_amplitudes.is_power_of_two() {
            return Err("dense-register tile size must be a nonzero power of two".into());
        }
        let fixed_format = FixedPointFormat::for_phase_register(qubits)?;
        Ok(Self {
            qubits,
            tile_amplitudes,
            fixed_format,
        })
    }

    pub fn tile(&self, tile_index: &BigUint) -> Option<(BigUint, u64)> {
        let start = tile_index * self.tile_amplitudes;
        let tile_bits = u64::from(self.tile_amplitudes.trailing_zeros());
        let count = if self.qubits < tile_bits { 1u64 << self.qubits } else { self.tile_amplitudes };
        if !self.contains_range(&start, count) { return None; }
        Some((start, count))
    }

    pub fn contains_range(&self, start: &BigUint, count: u64) -> bool {
        if count == 0 { return false; }
        let last = start + (count - 1);
        last.bits() <= self.qubits
    }

    pub fn butterfly_partner_tile(&self, tile_index: &BigUint, stage: u64) -> Option<BigUint> {
        if stage >= self.qubits || self.tile(tile_index).is_none() { return None; }
        let tile_bits = u64::from(self.tile_amplitudes.trailing_zeros());
        if stage < tile_bits { return Some(tile_index.clone()); }
        let tile_bit = usize::try_from(stage - tile_bits).ok()?;
        let partner = tile_index ^ (BigUint::one() << tile_bit);
        self.tile(&partner).map(|_| partner)
    }
}

pub struct DenseRegisterTape {
    root: String,
    layout: DenseRegisterLayout,
}

pub struct DenseRegisterTapeFixed {
    root: String,
    layout: DenseRegisterLayout,
    format: FixedPointFormat,
}

impl DenseRegisterTapeFixed {
    pub fn create(root: impl Into<String>, layout: DenseRegisterLayout, format: FixedPointFormat) -> Result<Self, String> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|error| format!("create fixed-point register tape: {error}"))?;
        Ok(Self { root, layout, format })
    }

    pub fn layout(&self) -> &DenseRegisterLayout { &self.layout }
    pub fn format(&self) -> &FixedPointFormat { &self.format }

    pub fn written_tiles(&self) -> Result<Vec<BigUint>, String> {
        written_tile_indices(&self.root, &self.layout)
    }

    fn tile_path(&self, tile_index: &BigUint) -> String {
        alloc::format!("{}/tile_{}.bin", self.root, tile_index.to_str_radix(16))
    }

    pub fn write_tile(&self, tile_index: &BigUint, amplitudes: &[FixedComplex]) -> Result<(), String> {
        let Some((_, expected)) = self.layout.tile(tile_index) else {
            return Err("fixed-point tile index is outside the logical register".into());
        };
        if usize::try_from(expected).ok() != Some(amplitudes.len()) {
            return Err(format!("fixed-point tile has {} amplitudes; expected {expected}", amplitudes.len()));
        }
        let cell_bytes = usize::try_from(self.format.w_bits.saturating_add(2).div_ceil(8))
            .map_err(|_| "fixed-point cell width exceeds host indexing")?.checked_mul(2)
            .ok_or("fixed-point tile width overflow")?;
        let mut bytes = Vec::with_capacity(amplitudes.len().checked_mul(cell_bytes).ok_or("fixed-point tile byte length overflow")?);
        for amplitude in amplitudes { bytes.extend_from_slice(&amplitude.to_bytes(&self.format)?); }
        let path = self.tile_path(tile_index);
        static NEXT_TEMP: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
        let temp = alloc::format!("{path}.tmp-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
        std::fs::write(&temp, bytes).map_err(|error| format!("write fixed-point tile: {error}"))?;
        std::fs::rename(&temp, &path).map_err(|error| format!("commit fixed-point tile: {error}"))
    }

    pub fn read_tile(&self, tile_index: &BigUint) -> Result<Vec<FixedComplex>, String> {
        let Some((_, count)) = self.layout.tile(tile_index) else {
            return Err("fixed-point tile index is outside the logical register".into());
        };
        let count = usize::try_from(count).map_err(|_| "fixed-point tile exceeds host indexing")?;
        let path = self.tile_path(tile_index);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(alloc::vec![FixedComplex { re: BigInt::zero(), im: BigInt::zero() }; count]);
            }
            Err(error) => return Err(format!("read fixed-point tile: {error}")),
        };
        let cell_bytes = usize::try_from(self.format.w_bits.saturating_add(2).div_ceil(8))
            .map_err(|_| "fixed-point cell width exceeds host indexing")?.checked_mul(2)
            .ok_or("fixed-point tile width overflow")?;
        if bytes.len() != count.checked_mul(cell_bytes).ok_or("fixed-point tile byte length overflow")? {
            return Err(format!("fixed-point tile {} has a corrupt byte length", tile_index));
        }
        bytes.chunks_exact(cell_bytes).map(|cell| FixedComplex::from_bytes(cell, &self.format)).collect()
    }

    pub fn read_qft_bin(&self, frequency: &BigUint) -> Result<Cx, String> {
        let physical_index = reverse_register_bits(frequency, self.layout.qubits)?;
        let tile_width = BigUint::from(self.layout.tile_amplitudes);
        let tile_index = &physical_index / &tile_width;
        let local_index = (&physical_index % tile_width).to_u64_digits().first().copied().unwrap_or(0);
        let tile = self.read_tile(&tile_index)?;
        tile.get(usize::try_from(local_index).map_err(|_| "QFT local index exceeds host indexing")?)
            .map(|cell| cell.decode(&self.format))
            .ok_or_else(|| "QFT frequency maps beyond its amplitude tile".into())
    }

    pub fn measure_qft_bin(&self, unit_sample: f64) -> Result<(BigUint, f64), String> {
        if !unit_sample.is_finite() || !(0.0..1.0).contains(&unit_sample) {
            return Err("Born sample must be finite and in [0, 1)".into());
        }
        let written = self.written_tiles()?;
        let mut total_mass = 0.0f64;
        for tile_index in &written {
            for cell in self.read_tile(tile_index)? {
                let decoded = cell.decode(&self.format);
                total_mass += decoded.norm2();
            }
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err("fixed dense-register spectrum has no finite Born mass".into());
        }
        let target = unit_sample * total_mass;
        let mut cumulative = 0.0;
        for tile_index in written {
            let Some((start, _)) = self.layout.tile(&tile_index) else { continue };
            for (local, cell) in self.read_tile(&tile_index)?.into_iter().enumerate() {
                cumulative += cell.decode(&self.format).norm2();
                if target < cumulative {
                    let physical = start + local;
                    return Ok((reverse_register_bits(&physical, self.layout.qubits)?, total_mass));
                }
            }
        }
        Err("fixed Born sampler failed to select an amplitude".into())
    }
}

fn reverse_register_bits(index: &BigUint, qubits: u64) -> Result<BigUint, String> {
    if index.bits() > qubits { return Err("register index lies outside the logical register".into()); }
    let qubits = usize::try_from(qubits).map_err(|_| "register bit reversal exceeds host indexing")?;
    let mut reversed = BigUint::zero();
    for bit in 0..qubits {
        if !((index >> bit) & BigUint::one()).is_zero() {
            reversed |= BigUint::one() << (qubits - bit - 1);
        }
    }
    Ok(reversed)
}

impl DenseRegisterTape {
    pub fn create(root: impl Into<String>, layout: DenseRegisterLayout) -> Result<Self, String> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|error| format!("create dense-register tape: {error}"))?;
        Ok(Self { root, layout })
    }

    pub fn layout(&self) -> &DenseRegisterLayout { &self.layout }

    pub fn written_tiles(&self) -> Result<Vec<BigUint>, String> {
        written_tile_indices(&self.root, &self.layout)
    }

    fn tile_path(&self, tile_index: &BigUint) -> String {
        alloc::format!("{}/tile_{}.bin", self.root, tile_index.to_str_radix(16))
    }

    pub fn write_tile(&self, tile_index: &BigUint, amplitudes: &[Cx]) -> Result<(), String> {
        let Some((_, expected)) = self.layout.tile(tile_index) else {
            return Err("dense-register tile index is outside the logical register".into());
        };
        if usize::try_from(expected).ok() != Some(amplitudes.len()) {
            return Err(format!("dense-register tile has {} amplitudes; expected {expected}", amplitudes.len()));
        }
        let bytes_len = amplitudes.len().checked_mul(16).ok_or("dense-register tile byte length overflow")?;
        let mut bytes = Vec::with_capacity(bytes_len);
        for amplitude in amplitudes {
            bytes.extend_from_slice(&amplitude.re.to_le_bytes());
            bytes.extend_from_slice(&amplitude.im.to_le_bytes());
        }
        let path = self.tile_path(tile_index);
        static NEXT_TEMP: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
        let temp = alloc::format!("{path}.tmp-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
        std::fs::write(&temp, bytes).map_err(|error| format!("write dense-register tile: {error}"))?;
        std::fs::rename(&temp, &path).map_err(|error| format!("commit dense-register tile: {error}"))
    }

    pub fn read_tile(&self, tile_index: &BigUint) -> Result<Vec<Cx>, String> {
        let Some((_, count)) = self.layout.tile(tile_index) else {
            return Err("dense-register tile index is outside the logical register".into());
        };
        let count = usize::try_from(count).map_err(|_| "dense-register tile exceeds host memory indexing")?;
        let path = self.tile_path(tile_index);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(alloc::vec![Cx::zero(); count]),
            Err(error) => return Err(format!("read dense-register tile: {error}")),
        };
        let cell_bytes = 16;
        if bytes.len() != count.checked_mul(cell_bytes).ok_or("dense-register tile byte length overflow")? {
            return Err(format!("dense-register tile {} has a corrupt byte length", tile_index));
        }
        Ok(bytes.chunks_exact(cell_bytes).map(|cell| Cx {
            re: f64::from_le_bytes(cell[..8].try_into().unwrap()),
            im: f64::from_le_bytes(cell[8..].try_into().unwrap()),
        }).collect())
    }

    pub fn read_qft_bin(&self, frequency: &BigUint) -> Result<Cx, String> {
        let physical_index = reverse_register_bits(frequency, self.layout.qubits)?;
        let tile_width = BigUint::from(self.layout.tile_amplitudes);
        let tile_index = &physical_index / &tile_width;
        let local_index = (&physical_index % tile_width).to_u64_digits().first().copied().unwrap_or(0);
        let tile = self.read_tile(&tile_index)?;
        tile.get(usize::try_from(local_index).map_err(|_| "QFT local index exceeds host indexing")?)
            .copied().ok_or_else(|| "QFT frequency maps beyond its amplitude tile".into())
    }

    pub fn measure_qft_bin(&self, unit_sample: f64) -> Result<(BigUint, f64), String> {
        if !unit_sample.is_finite() || !(0.0..1.0).contains(&unit_sample) {
            return Err("Born sample must be finite and in [0, 1)".into());
        }
        let mut total_mass = 0.0f64;
        let mut tile_index = BigUint::zero();
        while self.layout.tile(&tile_index).is_some() {
            for cell in self.read_tile(&tile_index)? { total_mass += cell.norm2(); }
            tile_index += BigUint::one();
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err("dense-register spectrum has no finite Born mass".into());
        }
        let target = unit_sample * total_mass;
        let mut cumulative = 0.0f64;
        tile_index = BigUint::zero();
        while let Some((start, _)) = self.layout.tile(&tile_index) {
            for (local, cell) in self.read_tile(&tile_index)?.into_iter().enumerate() {
                cumulative += cell.norm2();
                if target < cumulative {
                    let physical = start + local;
                    return Ok((reverse_register_bits(&physical, self.layout.qubits)?, total_mass));
                }
            }
            tile_index += BigUint::one();
        }
        Err("Born sampler failed to select an amplitude".into())
    }
}

fn written_tile_indices(root: &str, layout: &DenseRegisterLayout) -> Result<Vec<BigUint>, String> {
    let entries = std::fs::read_dir(root).map_err(|error| format!("scan dense-register tape: {error}"))?;
    let mut tiles = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("read dense-register tape entry: {error}"))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
        let Some(stem) = name.strip_prefix("tile_").and_then(|name| name.strip_suffix(".bin")) else { continue };
        let hex = if stem.starts_with('w') { stem.rsplit_once('_').map(|(_, hex)| hex).unwrap_or(stem) } else { stem };
        let Some(index) = BigUint::parse_bytes(hex.as_bytes(), 16) else { continue };
        if layout.tile(&index).is_some() { tiles.push(index); }
    }
    tiles.sort();
    Ok(tiles)
}

/// The exact QFT unitary, in place:
///   out[k] = (1/sqrt(M)) * sum_x in[x] * e^{-2 pi i k x / M}.
/// O(M log M), so the register reaches the factoring regime.
fn qft(buf: &mut [Cx]) {
    let m = buf.len();
    let mut j: usize = 0;
    for i in 0..m {
        if i < j { buf.swap(i, j); }
        let mut bit = m >> 1;
        while bit != 0 && (j & bit) != 0 { j ^= bit; bit >>= 1; }
        j |= bit;
    }
    let mut len = 2usize;
    while len <= m {
        let half = len >> 1;
        let wv: Vec<Cx> = (0..half)
            .map(|t| {
                let ang = -2.0 * core::f64::consts::PI * (t as f64) / (len as f64);
                Cx::new(libm::cos(ang), libm::sin(ang))
            })
            .collect();
        let mut start = 0usize;
        while start < m {
            for t in 0..half {
                let u = buf[start + t];
                let v = buf[start + t + half] * wv[t];
                buf[start + t] = u + v;
                buf[start + t + half] = u - v;
            }
            start += len;
        }
        len <<= 1;
    }
    let inv = 1.0 / libm::sqrt(m as f64);
    for a in buf.iter_mut() { *a = a.scale(inv); }
}

fn gcd_big(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() { let t = b.clone(); b = &a % &b; a = t; }
    a
}

fn powm_big(mut base: BigUint, mut e: BigUint, m: &BigUint) -> BigUint {
    let mut r: BigUint = BigUint::one() % m;
    base = base % m;
    while !e.is_zero() {
        if &e & BigUint::one() == BigUint::one() {
            r = (&r * &base) % m;
        }
        e >>= 1;
        base = (&base * &base) % m;
    }
    r
}

/// Classical reference period used to prepare the modeled comb and report
/// its reference denominator; the spectral transform does not infer it.
fn true_period_big(a: &BigUint, n: &BigUint) -> BigUint {
    let mut v: BigUint = BigUint::one() % n;
    let mut r = BigUint::zero();
    loop {
        r += BigUint::one();
        v = (&v * a) % n;
        if v == BigUint::one() { return r; }
        if &r > n { return BigUint::zero(); }
    }
}

struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.0 = x; x
    }
    fn unit(&mut self) -> f64 { ((self.next() >> 11) as f64) / 9007199254740992.0 }
}

/// Continued-fraction convergents of k/m — BigUint lift so it can carry
/// convergents larger than u64.
fn convergents_big(mut k: BigUint, mut m: BigUint) -> Vec<(BigUint, BigUint)> {
    let mut out: Vec<(BigUint, BigUint)> = Vec::new();
    let mut p_prev: BigUint = BigUint::zero();
    let mut p_curr: BigUint = BigUint::one();
    let mut q_prev: BigUint = BigUint::one();
    let mut q_curr: BigUint = BigUint::zero();
    while !m.is_zero() {
        let a = &k / &m;
        let p_next = &a * &p_curr + &p_prev;
        let q_next = &a * &q_curr + &q_prev;
        out.push((p_next.clone(), q_next.clone()));
        p_prev = p_curr; p_curr = p_next;
        q_prev = q_curr; q_curr = q_next;
        let rem = &k % &m;
        k = m; m = rem;
    }
    out
}

enum Attempt {
    Factors { r: BigUint, s: BigUint, r0: BigUint, p: BigUint, q: BigUint, via: String },
    Degenerate(String),
    NoReadout,
}/// One measured k → winding → CF → period lift → algebraic closure.
/// All arithmetic on BigUint, so convergents larger than u64 are fine.
fn attempt_big(k: usize, m_pow2: usize, a: &BigUint, n: &BigUint) -> Attempt {
    attempt_big_wide(BigUint::from(k), BigUint::from(m_pow2), a, n)
}

fn attempt_big_wide(k_big: BigUint, m_big: BigUint, a: &BigUint, n: &BigUint) -> Attempt {
    let two: BigUint = BigUint::from(2u32);
    let one: BigUint = BigUint::one();
    for (s, r0) in convergents_big(k_big.clone(), m_big.clone()) {
        if r0 <= one { continue; }
        if &r0 > n { continue; }
        let r = r0.clone();
        if powm_big(a.clone(), r.clone(), n) != one { continue; }
        if &r % &two != BigUint::zero() {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> period r={} is odd — a^(r/2) has no half-step; advancing the coprime base",
                r0, r));
        }
        let half = &r / &two;
        let xh = powm_big(a.clone(), half, n);
        if &xh + &one == *n {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> r={} but a^(r/2) == -1 (mod N) — no split this base; advancing",
                r0, r));
        }
        let g1 = gcd_big(&xh - &one, n.clone());
        let g2 = gcd_big(&xh + &one, n.clone());
        let (p, q, via) = if &g1 > &one && &g1 < n {
            (g1.clone(), n / &g1, format!("gcd(a^(r/2) - 1, N) = {}", g1))
        } else if &g2 > &one && &g2 < n {
            (g2.clone(), n / &g2, format!("gcd(a^(r/2) + 1, N) = {}", g2))
        } else {
            return Attempt::Degenerate(format!(
                "CF: s/{} -> r={} but both gcd closures trivial; advancing", r0, r));
        };
        return Attempt::Factors { r, s, r0, p, q, via };
    }
    Attempt::NoReadout
}

pub fn close_wide_phase_readout(k: &BigUint, qubits: u64, a: &BigUint, n: &BigUint) -> Result<Option<(BigUint, BigUint, BigUint)>, String> {
    if n < &BigUint::from(2u8) || a.is_zero() || k.bits() > qubits { return Err("phase closure requires valid N, base and register bin".into()); }
    let shift = usize::try_from(qubits).map_err(|_| "phase-register denominator exceeds BigUint shift indexing")?;
    match attempt_big_wide(k.clone(), BigUint::one() << shift, a, n) {
        Attempt::Factors { r, p, q, .. } => Ok(Some((r, p, q))),
        Attempt::Degenerate(message) => Err(message),
        Attempt::NoReadout => Ok(None),
    }
}

/// The target register survives each control measurement. Its keys are modular
/// residues, with complex amplitudes at the requested fixed precision.
pub struct RecycledPhaseState {
    target: alloc::collections::BTreeMap<BigUint, FixedComplex>,
    powers: Vec<BigUint>,
    modulus: BigUint,
    format: FixedPointFormat,
    qubits: u64,
    remaining: usize,
    pub readout: BigUint,
    pub peak_support: usize,
}

/// One sorted modular image is reused by both measurement passes. Amplitude
/// references share the resident state; output branches occupy one batch.
struct PreparedRecycledBranches<'a> {
    state: &'a RecycledPhaseState,
    image: Vec<(BigUint, &'a FixedComplex)>,
    feedback: FixedComplex,
}

impl PreparedRecycledBranches<'_> {
    fn visit<F, O>(&self, batch: usize, mix: &mut F, mut observe: O) -> Result<usize, String>
    where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String>,
          O: FnMut(&[BigUint], &[FixedComplex], &[FixedComplex]) -> Result<(), String> {
        if batch == 0 { return Err("QPE branch batch must be nonempty".into()); }
        let mut original = self.state.target.iter().peekable();
        let mut image = self.image.iter().peekable();
        let capacity = batch.min(self.state.target.len().saturating_mul(2));
        let mut keys = Vec::with_capacity(capacity);
        let mut low = Vec::with_capacity(capacity);
        let mut high = Vec::with_capacity(capacity);
        let zero = FixedComplex { re: BigInt::zero(), im: BigInt::zero() };
        let mut count = 0usize;
        while original.peek().is_some() || image.peek().is_some() {
            let ordering = match (original.peek(), image.peek()) {
                (Some((left, _)), Some((right, _))) => (*left).cmp(right),
                (Some(_), None) => core::cmp::Ordering::Less,
                (None, Some(_)) => core::cmp::Ordering::Greater,
                (None, None) => break,
            };
            match ordering {
                core::cmp::Ordering::Less => {
                    let (key, amplitude) = original.next().unwrap();
                    keys.push(key.clone()); low.push(amplitude.clone()); high.push(zero.clone());
                }
                core::cmp::Ordering::Greater => {
                    let (key, amplitude) = image.next().unwrap();
                    keys.push(key.clone()); low.push(zero.clone()); high.push((*amplitude).clone());
                }
                core::cmp::Ordering::Equal => {
                    let (key, amplitude) = original.next().unwrap();
                    let (_, moved) = image.next().unwrap();
                    keys.push(key.clone()); low.push(amplitude.clone()); high.push((*moved).clone());
                }
            }
            count = count.checked_add(1).ok_or("QPE support count overflow")?;
            if keys.len() == batch {
                mix(&mut low, &mut high, &self.feedback, &self.state.format)?;
                observe(&keys, &low, &high)?;
                keys.clear(); low.clear(); high.clear();
            }
        }
        if !keys.is_empty() {
            mix(&mut low, &mut high, &self.feedback, &self.state.format)?;
            observe(&keys, &low, &high)?;
        }
        Ok(count)
    }
}

impl RecycledPhaseState {
    fn prepare_branches(&self) -> Result<PreparedRecycledBranches<'_>, String> {
        if self.remaining == 0 { return Err("QPE has no unmeasured phase bits".into()); }
        let multiplier = &self.powers[self.remaining - 1];
        let mut image: Vec<_> = self.target.iter()
            .map(|(residue, amplitude)| ((residue * multiplier) % &self.modulus, amplitude)).collect();
        image.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let feedback = FixedComplex::qft_twiddle(&self.readout, self.qubits - self.remaining as u64, false, &self.format)?;
        Ok(PreparedRecycledBranches { state: self, image, feedback })
    }

    /// Sum both Born masses batch by batch, then reconstruct the chosen branch
    /// with the same global normalization used by the materialized control.
    fn measure_next_batched<F>(&mut self, batch: usize, rng: &mut u64, mix: &mut F) -> Result<(bool, BigUint, BigUint), String>
    where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String> {
        // A register that fits in one batch needs only one butterfly pass.
        if self.target.len() <= batch / 2 {
            let (keys, low, high) = self.branches(batch, mix)?;
            let zero_mass = fixed_born_mass(&low);
            let one_mass = fixed_born_mass(&high);
            let bit = phase_born_rank(&(&zero_mass + &one_mass), || phase_random_word(rng))? >= zero_mass;
            self.commit(keys, if bit { high } else { low }, bit)?;
            return Ok((bit, zero_mass, one_mass));
        }
        let prepared = self.prepare_branches()?;
        let mut zero_mass = BigUint::zero();
        let mut one_mass = BigUint::zero();
        let mut largest = [BigInt::zero(), BigInt::zero()];
        let support = prepared.visit(batch, mix, |_, low, high| {
            zero_mass += fixed_born_mass(low);
            one_mass += fixed_born_mass(high);
            for (maximum, cells) in largest.iter_mut().zip([low, high]) {
                for cell in cells {
                    *maximum = maximum.clone().max(cell.re.abs()).max(cell.im.abs());
                }
            }
            Ok(())
        })?;
        let total = &zero_mass + &one_mass;
        let bit = phase_born_rank(&total, || phase_random_word(rng))? >= zero_mass;
        let maximum = &largest[usize::from(bit)];
        if maximum.is_zero() { return Err("QPE selected a zero Born mass branch".into()); }
        let scale = self.format.scale();
        let mut target = alloc::collections::BTreeMap::new();
        prepared.visit(batch, mix, |keys, low, high| {
            for (key, cell) in keys.iter().zip(if bit { high } else { low }) {
                let cell = FixedComplex {
                    re: fixed_round_div(&cell.re * &scale, maximum),
                    im: fixed_round_div(&cell.im * &scale, maximum),
                };
                if !cell.re.is_zero() || !cell.im.is_zero() { target.insert(key.clone(), cell); }
            }
            Ok(())
        })?;
        drop(prepared);
        self.target = target;
        self.peak_support = self.peak_support.max(support);
        if bit { self.readout |= BigUint::one() << (self.qubits - self.remaining as u64) as usize; }
        self.remaining -= 1;
        Ok((bit, zero_mass, one_mass))
    }
    pub fn new(n: &BigUint, base: &BigUint, qubits: u64, format: &FixedPointFormat) -> Result<Self, String> {
        if n < &BigUint::from(2u8) || qubits == 0 || gcd_big(base.clone(), n.clone()) != BigUint::one() {
            return Err("recycled QPE requires N >= 2, a coprime base and a nonempty phase register".into());
        }
        let count = usize::try_from(qubits).map_err(|_| "QPE powers exceed host indexing")?;
        let mut powers = Vec::with_capacity(count);
        let mut power = base % n;
        for _ in 0..count {
            powers.push(power.clone());
            power = (&power * &power) % n;
        }
        let target = [(BigUint::one(), FixedComplex { re: format.scale(), im: BigInt::zero() })].into_iter().collect();
        Ok(Self { target, powers, modulus: n.clone(), format: format.clone(), qubits,
            remaining: count, readout: BigUint::zero(), peak_support: 1 })
    }

    /// Controlled modular multiplication, feedback rotation, and the two
    /// Hadamard branches. Both outputs share the same omitted normalization.
    pub fn branches<F>(&self, tile_amplitudes: usize, mix: &mut F) -> Result<(Vec<BigUint>, Vec<FixedComplex>, Vec<FixedComplex>), String>
    where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String> {
        if self.remaining == 0 || tile_amplitudes == 0 { return Err("QPE step requires an unmeasured bit and a nonempty tile".into()); }
        let multiplier = &self.powers[self.remaining - 1];
        // Multiplication by a coprime base is a permutation. Sort its sparse
        // image once, then merge both ordered streams without another tree
        // or a cloned amplitude map.
        let mut permuted: Vec<_> = self.target.iter()
            .map(|(residue, amplitude)| ((residue * multiplier) % &self.modulus, amplitude)).collect();
        permuted.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let capacity = self.target.len().checked_add(permuted.len()).ok_or("QPE support size overflow")?;
        let mut keys = Vec::with_capacity(capacity);
        let mut low = Vec::with_capacity(capacity);
        let mut high = Vec::with_capacity(capacity);
        let zero = FixedComplex { re: BigInt::zero(), im: BigInt::zero() };
        let mut original = self.target.iter().peekable();
        let mut image = permuted.into_iter().peekable();
        while original.peek().is_some() || image.peek().is_some() {
            let ordering = match (original.peek(), image.peek()) {
                (Some((left, _)), Some((right, _))) => (*left).cmp(right),
                (Some(_), None) => core::cmp::Ordering::Less,
                (None, Some(_)) => core::cmp::Ordering::Greater,
                (None, None) => break,
            };
            match ordering {
                core::cmp::Ordering::Less => {
                    let (key, amplitude) = original.next().unwrap();
                    keys.push(key.clone()); low.push(amplitude.clone()); high.push(zero.clone());
                }
                core::cmp::Ordering::Greater => {
                    let (key, amplitude) = image.next().unwrap();
                    keys.push(key); low.push(zero.clone()); high.push(amplitude.clone());
                }
                core::cmp::Ordering::Equal => {
                    let (key, amplitude) = original.next().unwrap();
                    let (_, moved) = image.next().unwrap();
                    keys.push(key.clone()); low.push(amplitude.clone()); high.push(moved.clone());
                }
            }
        }
        let output_bit = self.qubits - self.remaining as u64;
        let feedback = FixedComplex::qft_twiddle(&self.readout, output_bit, false, &self.format)?;
        for (low_tile, high_tile) in low.chunks_mut(tile_amplitudes).zip(high.chunks_mut(tile_amplitudes)) {
            mix(low_tile, high_tile, &feedback, &self.format)?;
        }
        Ok((keys, low, high))
    }

    pub fn commit(&mut self, keys: Vec<BigUint>, amplitudes: Vec<FixedComplex>, bit: bool) -> Result<(), String> {
        if self.remaining == 0 || keys.len() != amplitudes.len() { return Err("QPE commit has inconsistent register dimensions".into()); }
        let largest = amplitudes.iter().flat_map(|cell| [cell.re.abs(), cell.im.abs()]).max().unwrap_or_default();
        if largest.is_zero() { return Err("QPE selected a zero Born mass branch".into()); }
        self.peak_support = self.peak_support.max(keys.len());
        let scale = self.format.scale();
        self.target = keys.into_iter().zip(amplitudes).filter_map(|(key, cell)| {
            let cell = FixedComplex {
                re: fixed_round_div(cell.re * &scale, &largest),
                im: fixed_round_div(cell.im * &scale, &largest),
            };
            (!cell.re.is_zero() || !cell.im.is_zero()).then_some((key, cell))
        }).collect();
        self.peak_support = self.peak_support.max(self.target.len());
        if bit { self.readout |= BigUint::one() << (self.qubits - self.remaining as u64) as usize; }
        self.remaining -= 1;
        Ok(())
    }

    pub fn write_target(&self, tape: &DenseRegisterTapeFixed) -> Result<(), String> {
        if tape.format() != &self.format { return Err("QPE target tape precision differs from the live register".into()); }
        let width = BigUint::from(tape.layout().tile_amplitudes);
        let mut tiles = alloc::collections::BTreeMap::new();
        for (residue, cell) in &self.target {
            let index = residue / &width;
            let lane = (residue % &width).to_u64_digits().first().copied().unwrap_or(0) as usize;
            let (_, count) = tape.layout().tile(&index).ok_or("QPE residue is outside the target tape")?;
            let tile = tiles.entry(index).or_insert_with(|| alloc::vec![FixedComplex { re: BigInt::zero(), im: BigInt::zero() }; count as usize]);
            tile[lane] = cell.clone();
        }
        for (index, tile) in tiles { tape.write_tile(&index, &tile)?; }
        Ok(())
    }
}

pub fn fixed_born_mass(amplitudes: &[FixedComplex]) -> BigUint {
    amplitudes.iter().fold(BigUint::zero(), |mass, cell| {
        mass + (&cell.re * &cell.re + &cell.im * &cell.im).to_biguint().unwrap()
    })
}

fn phase_random_word(rng: &mut u64) -> u64 {
    *rng ^= *rng << 13; *rng ^= *rng >> 7; *rng ^= *rng << 17;
    *rng
}

/// Integer inverse-CDF rank without a fixed-width quantile grid. Rejection
/// avoids a modulo bias when the total Born mass is not a power of two.
fn phase_born_rank<F>(total: &BigUint, mut random_word: F) -> Result<BigUint, String>
where F: FnMut() -> u64 {
    if total.is_zero() { return Err("cannot sample zero Born mass".into()); }
    let bits = usize::try_from((total - BigUint::one()).bits()).map_err(|_| "Born mass exceeds host limb indexing")?;
    if bits == 0 { return Ok(BigUint::zero()); }
    let mask = (BigUint::one() << bits) - BigUint::one();
    loop {
        let mut rank = BigUint::zero();
        for bit in (0..bits).step_by(64) { rank |= BigUint::from(random_word()) << bit; }
        rank &= &mask;
        if &rank < total { return Ok(rank); }
    }
}

fn fresh_phase_base(n: &BigUint, rng: &mut u64) -> BigUint {
    let bits = n.bits() as usize;
    let mask = (BigUint::one() << bits) - BigUint::one();
    let upper = n - BigUint::one();
    loop {
        let mut candidate = BigUint::zero();
        for bit in (0..bits).step_by(64) { candidate |= BigUint::from(phase_random_word(rng)) << bit; }
        candidate &= &mask;
        if candidate >= BigUint::from(2u8) && candidate < upper { return candidate; }
    }
}

/// CPU control for the CUDA butterflies, including their intermediate rounding.
pub fn recycled_phase_mix_cpu(low: &mut [FixedComplex], high: &mut [FixedComplex], feedback: &FixedComplex, format: &FixedPointFormat) -> Result<(), String> {
    if low.len() != high.len() { return Err("QPE branch buffers have inconsistent lengths".into()); }
    for (a, b) in low.iter_mut().zip(high) {
        let rotated = FixedComplex { re: -&b.re, im: -&b.im }.mul(feedback, format).half();
        let rotated = FixedComplex { re: rotated.re * -2, im: rotated.im * -2 };
        let sum = FixedComplex { re: &a.re + &rotated.re, im: &a.im + &rotated.im }.half();
        let difference = FixedComplex { re: &a.re - &rotated.re, im: &a.im - &rotated.im }.half();
        *a = sum;
        *b = difference;
    }
    Ok(())
}

pub fn measure_recycled_phase<F>(state: &mut RecycledPhaseState, tile_amplitudes: usize, rng: &mut u64, mut mix: F) -> Result<String, String>
where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String> {
    let mut trace = String::new();
    measure_recycled_phase_stream(state, tile_amplitudes, rng, &mut mix, |line| {
        trace.push_str(line);
        Ok(())
    })?;
    Ok(trace)
}

/// Emit each completed measurement before advancing the next modular power.
/// A live tape therefore records progress even if a later step cannot finish.
pub fn measure_recycled_phase_stream<F, O>(state: &mut RecycledPhaseState, tile_amplitudes: usize, rng: &mut u64, mut mix: F, mut observe: O) -> Result<(), String>
where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String>,
      O: FnMut(&str) -> Result<(), String> {
    observe("power_bit\tmeasured_bit\tzero_mass\tone_mass\tretained_residues\n")?;
    while state.remaining != 0 {
        let power_bit = state.remaining - 1;
        let (bit, zero_mass, one_mass) = state.measure_next_batched(tile_amplitudes, rng, &mut mix)?;
        observe(&format!("{power_bit}\t{}\t{zero_mass}\t{one_mass}\t{}\n", u8::from(bit), state.target.len()))?;
    }
    Ok(())
}

/// Combine only denominators justified by a measured phase. Modular powers
/// certify the resulting LCM; no multiples of a candidate are walked.
pub struct PhaseReadoutAccumulator {
    denominator: BigUint,
    source_base: Option<(BigUint, BigUint)>,
}

impl Default for PhaseReadoutAccumulator {
    fn default() -> Self { Self { denominator: BigUint::one(), source_base: None } }
}

impl PhaseReadoutAccumulator {
    pub fn close(&mut self, k: &BigUint, qubits: u64, base: &BigUint, n: &BigUint) -> Result<Option<(BigUint, BigUint, BigUint)>, String> {
        if n < &BigUint::from(2u8) || base.is_zero() || k.bits() > qubits { return Err("phase closure requires valid N, base and register bin".into()); }
        // Denominators are evidence about one modular orbit. Reusing this
        // object for another source or base must begin a fresh accumulation.
        if self.source_base.as_ref().map(|(source, orbit_base)| source != n || orbit_base != base).unwrap_or(true) {
            self.denominator = BigUint::one();
            self.source_base = Some((n.clone(), base.clone()));
        }
        if k.is_zero() { return Ok(None); }
        if let Some(result) = close_wide_phase_readout(k, qubits, base, n)? { return Ok(Some(result)); }
        let m = BigUint::one() << usize::try_from(qubits).map_err(|_| "phase denominator exceeds host indexing")?;
        for (s, r) in convergents_big(k.clone(), m.clone()) {
            if r <= BigUint::one() || &r > n { continue; }
            let left = k * &r;
            let right = s * &m;
            let error = if left >= right { left - right } else { right - left };
            // Only a denominator whose rational phase lies within one bin
            // contributes. Early, coarse convergents carry no such evidence.
            if error > r { continue; }
            let gcd = gcd_big(self.denominator.clone(), r.clone());
            let candidate = &self.denominator / gcd * r;
            if &candidate > n { continue; }
            self.denominator = candidate;
            if base.modpow(&self.denominator, n).is_one() {
                let period = self.denominator.clone();
                if (&period & BigUint::one()).is_one() { return Err(format!("certified period {period} is odd")); }
                let half = base.modpow(&(&period >> 1usize), n);
                if half.is_one() || &half + BigUint::one() == *n { return Err(format!("certified period {period} gives a trivial half-period split")); }
                for difference in [&half - BigUint::one(), &half + BigUint::one()] {
                    let p = gcd_big(difference, n.clone());
                    if p > BigUint::one() && &p < n { return Ok(Some((period, p.clone(), n / p))); }
                }
            }
            break;
        }
        Ok(None)
    }
}

pub fn recycled_phase_report<F>(n: BigUint, mut base: BigUint, tile_amplitudes: u64, tape_root: &str, shots: u32, mut mix: F) -> Result<String, String>
where F: FnMut(&mut [FixedComplex], &mut [FixedComplex], &FixedComplex, &FixedPointFormat) -> Result<(), String> {
    if n.bits() < 128 || shots == 0 { return Err("QPE factor execution requires a semiprime of at least 128 bits and a positive shot budget".into()); }
    let qubits = n.bits().checked_mul(2).and_then(|bits| bits.checked_add(8)).ok_or("phase register width overflow")?;
    let format = FixedPointFormat::for_modulus(&n)?;
    let layout = DenseRegisterLayout::new(n.bits(), tile_amplitudes)?;
    let tile_len = usize::try_from(tile_amplitudes).map_err(|_| "QPE tile exceeds host indexing")?;
    if base < BigUint::from(2u8) { base = BigUint::from(2u8); }
    let mut rng = n.to_u64_digits().first().copied().unwrap_or(0) ^ 0x9E37_79B9_7F4A_7C15;
    if rng == 0 { rng = 1; }
    let root = std::path::Path::new(tape_root);
    std::fs::create_dir_all(root).map_err(|error| format!("create QPE tape root: {error}"))?;
    if root.join("configuration.txt").exists() || root.join("shot_1").exists() {
        return Err(format!("QPE tape root {tape_root} already carries a run; use a fresh directory"));
    }
    std::fs::write(root.join("configuration.txt"), format!("N={n}\ninitial_base={base}\nphase_bits={qubits}\nW={}\nbatch_amplitudes={tile_amplitudes}\nshots={shots}\n", format.w_bits))
        .map_err(|error| format!("write QPE configuration: {error}"))?;
    let mut report = format!("Recycled control QPE with fixed complex amplitudes\nN = {n}\nphase bits = {qubits}\nW = {}\nCUDA batch amplitudes = {tile_amplitudes}\ntape = {tape_root}\n", format.w_bits);
    let mut accumulated = PhaseReadoutAccumulator::default();
    for shot in 1..=shots {
        let divisor = gcd_big(base.clone(), n.clone());
        let result = if divisor > BigUint::one() && divisor < n {
            report.push_str(&format!("base {base} shares divisor {divisor} with N\n"));
            Some((BigUint::zero(), divisor.clone(), &n / divisor))
        } else if divisor != BigUint::one() {
            base = fresh_phase_base(&n, &mut rng);
            accumulated = PhaseReadoutAccumulator::default();
            continue;
        } else {
            let shot_root = root.join(format!("shot_{shot}"));
            // Preserve previous runs; a requested path always binds one run.
            std::fs::create_dir(&shot_root).map_err(|error| format!("create QPE shot tape {}: {error}", shot_root.display()))?;
            let mut state = RecycledPhaseState::new(&n, &base, qubits, &format)?;
            use std::io::Write;
            let mut trace = std::fs::OpenOptions::new().write(true).create_new(true)
                .open(shot_root.join("measurements.tsv")).map_err(|error| format!("create live QPE measurements: {error}"))?;
            measure_recycled_phase_stream(&mut state, tile_len, &mut rng, &mut mix, |line| {
                trace.write_all(line.as_bytes()).map_err(|error| format!("write live QPE measurement: {error}"))
            })?;
            trace.sync_all().map_err(|error| format!("sync QPE measurements: {error}"))?;
            let tape = DenseRegisterTapeFixed::create(shot_root.join("target").to_string_lossy().into_owned(), layout.clone(), format.clone())?;
            state.write_target(&tape)?;
            report.push_str(&format!("shot {shot}: base={base}, k={}, measured bits={qubits}, peak residue support={}\n", state.readout, state.peak_support));
            match accumulated.close(&state.readout, qubits, &base, &n) {
                Ok(result) => result,
                Err(message) => {
                    report.push_str(&format!("readout closure: {message}\n"));
                    base = fresh_phase_base(&n, &mut rng);
                    accumulated = PhaseReadoutAccumulator::default();
                    None
                }
            }
        };
        if let Some((period, p, q)) = result {
            if native_numeral::multiply_via_word(&p, &q) != n || !native_numeral::syzygy_preserves(&n, &p, &q) { return Err("measured factor pair failed word product closure".into()); }
            report.push_str(&format!("period = {period}\nfactors = {p} × {q}\nverified word product = true\n{}", native_numeral::factor_words_line(&p, &q)));
            std::fs::write(root.join("report.txt"), &report).map_err(|error| format!("write QPE closure report: {error}"))?;
            return Ok(report);
        }
    }
    report.push_str("shot budget completed\n");
    std::fs::write(root.join("report.txt"), &report).map_err(|error| format!("write QPE measurement report: {error}"))?;
    Ok(report)
}

pub struct PhaseUnbraidResult {
    pub tape_dir: Option<String>,
    pub n_val: BigUint,
    pub a_used: BigUint,
    pub n_qubits: usize,
    pub m: usize,
    pub total_shots: u32,
    pub shot_k: Option<usize>,
    pub certified_r: Option<BigUint>,
    pub true_r: BigUint,
    pub factors: Option<(BigUint, BigUint)>,
    pub trace: String,
}

/// The phase-based unbraid, BigUint lift. No static caps on q, a_tries,
/// a_shots — q is derived from bits(N) and the register grows to host
/// memory. Streaming FFT: when M exceeds the configured in-memory
/// amplitude budget (`mem_cap`, default 1<<22 amplitudes ≈ 64 MiB),
/// the comb is staged in blocks; per-block exact QFT runs entirely in
/// memory, then the per-cell probability mass is sampled by inverse-CDF
/// over block totals (a single full-width f64 block_cdf, not a
/// BigUint cumulative vector — f64 mass is exact enough for one shot).
pub fn run_phase_unbraid_big(
    n_val: BigUint,
    a0: BigUint,
    max_shots: u32,
    mem_cap: usize,
    tape_dir: Option<String>,
) -> Result<PhaseUnbraidResult, String> {
    run_phase_unbraid_big_with_qft(n_val, a0, max_shots, mem_cap, tape_dir, false, |_, _, buffer| {
        qft(buffer);
        Ok(())
    })
}

/// Execute the baked phase-membrane pipeline with a caller-supplied spectral
/// transform. Hosted GPU executors use this seam for the actual QFT kernel;
/// the CPU reference remains the default backend above.
pub fn run_phase_unbraid_big_with_qft<F>(
    n_val: BigUint,
    a0: BigUint,
    max_shots: u32,
    mem_cap: usize,
    tape_dir: Option<String>,
    prepare_on_gpu: bool,
    mut transform: F,
) -> Result<PhaseUnbraidResult, String>
where
    F: FnMut(usize, usize, &mut [Cx]) -> Result<(), String>,
{
    let _tape_dir_init_keepalive: Option<String> = tape_dir.clone().map(|s| { let _ = std::fs::create_dir_all(&s); s });
    let _tape_dir_init_keepalive = _tape_dir_init_keepalive;
    let mut trace = String::new();
    if n_val < BigUint::from(4u32) {
        return Err("N < 4 has no nontrivial two-factor closure".into());
    }
    let two: BigUint = BigUint::from(2u32);
    if &n_val % &two == BigUint::zero() {
        let q = &n_val / &two;
        trace.push_str("N even: peeled directly (p=2); the phase register below assumes odd N\n");
        return Ok(PhaseUnbraidResult {
            tape_dir: _tape_dir_init_keepalive.clone(), n_val, a_used: BigUint::zero(), n_qubits: 0, m: 0, total_shots: 0,
            shot_k: None, certified_r: Some(BigUint::one()), true_r: BigUint::one(),
            factors: Some((two, q)), trace,
        });
    }
    let bits_n = n_val.bits() as usize;
    let q = 2 * bits_n + 8;
    if q >= usize::BITS as usize {
        return Err(format!("phase register requires 2^{q} amplitudes, exceeding host indexing"));
    }
    let m: usize = 1usize << q;
    let mem_cap = mem_cap.max(1usize << 10);
    let block_amps = mem_cap.min(m);
    if (block_amps & (block_amps - 1)) != 0 {
        return Err(format!("mem_cap must be a power of two (got {})", block_amps));
    }
    if prepare_on_gpu && block_amps != m {
        return Err(format!("CUDA QFT requires the complete register in one transform (M={m}, mem_cap={block_amps}); independent block FFTs are not a full-register QFT"));
    }
    let mut a = if a0 < BigUint::from(2u32) { BigUint::from(2u32) } else { a0 };
    let seed_mix = (&n_val % BigUint::from(u64::MAX)).iter_u64_digits().next().unwrap_or(0);
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15 ^ seed_mix);
    let mut total_shots: u32 = 0;
    loop {
        if total_shots >= max_shots { break; }
        let g = gcd_big(a.clone(), n_val.clone());
        if g != BigUint::one() {
            trace.push_str(&format!("  a={}: gcd(a,N)={} — trivial factor found directly, advancing\n", a, g));
            if &g != &n_val { return Ok(PhaseUnbraidResult {
                tape_dir: _tape_dir_init_keepalive.clone(), n_val: n_val.clone(), a_used: a.clone(), n_qubits: q, m, total_shots,
                shot_k: None, certified_r: Some(BigUint::zero()), true_r: BigUint::zero(),
                factors: Some((g.clone(), &n_val / &g)), trace,
            }); }
            a += BigUint::one(); continue;
        }
        let r_true = if prepare_on_gpu { BigUint::zero() } else { true_period_big(&a, &n_val) };
        if !prepare_on_gpu && r_true.is_zero() {
            trace.push_str(&format!("  a={}: period not found within N steps; advancing\n", a));
            a += BigUint::one(); continue;
        }
        let tape_dir_buf: Option<String> = _tape_dir_init_keepalive.clone();
        let _keep_tape = tape_dir_buf.clone();
        let r_true_us = if prepare_on_gpu { 1 } else { r_true.to_u64_digits().first().copied().unwrap_or(1).max(1) as usize };
        let l = if prepare_on_gpu { m } else { m / r_true_us };
        if !prepare_on_gpu && l < 2 {
            trace.push_str(&format!(
                "  a={}: comb degenerates (L={} < 2); advancing coprime base\n", a, l));
            a += BigUint::one(); continue;
        }
        let amp = 1.0 / libm::sqrt(l as f64);
        let n_blocks = m / block_amps;
        let mut a_shots: u32 = 0;
        loop {
            if a_shots >= 8 || total_shots >= max_shots { break; }
            a_shots += 1;
            total_shots += 1;
            // First sweep: per-block mass. If tape_dir is set, per-cell
            // QFT-output norms are spilled to <tape_dir>/block_<bi>.bin as
            // f64 little-endian; peak RAM stays at one block_amps buffer
            // regardless of M = 2^q.
            let mut block_mass: Vec<f64> = alloc::vec![0.0f64; n_blocks];
            for bi in 0..n_blocks {
                let start = bi * block_amps;
                let end = start + block_amps;
                let mut buf: Vec<Cx> = alloc::vec![Cx::zero(); block_amps];
                if !prepare_on_gpu {
                    let mut x = start;
                    while x < end { buf[x - start] = Cx::new(amp, 0.0); x += r_true_us; }
                }
                transform(start, m, &mut buf)?;
                let mut s = 0.0f64;
                let mut per_cell: Vec<f64> = if tape_dir_buf.is_some() {
                    alloc::vec![0.0f64; block_amps]
                } else {
                    Vec::new()
                };
                for (i, c) in buf.iter().enumerate() {
                    let n2 = c.norm2();
                    s += n2;
                    if !per_cell.is_empty() { per_cell[i] = n2; }
                }
                block_mass[bi] = s;
                if let Some(ref td) = tape_dir_buf {
                    let path = alloc::format!("{}/block_{:08}.bin", td, bi);
                    if let Ok(mut f) = std::fs::File::create(&path) {
                        use std::io::Write;
                        let bytes = unsafe {
                            core::slice::from_raw_parts(
                                per_cell.as_ptr() as *const u8,
                                per_cell.len() * core::mem::size_of::<f64>(),
                            )
                        };
                        let _ = f.write_all(bytes);
                    }
                }
            }
            let mut block_cdf: Vec<f64> = alloc::vec![0.0f64; n_blocks];
            let mut acc = 0.0f64;
            for (i, m) in block_mass.iter().enumerate() { acc += m; block_cdf[i] = acc; }
            let total_mass = *block_cdf.last().unwrap_or(&0.0);
            if total_mass <= 0.0 {
                trace.push_str(&format!("  shot {}: zero total Born mass — re-measuring\n", total_shots));
                continue;
            }
            let target = rng.unit() * total_mass;
            let mut bi = n_blocks - 1;
            for (i, c) in block_cdf.iter().enumerate() { if target < *c { bi = i; break; } }
            let start = bi * block_amps;
            let local_target = rng.unit() * block_mass[bi];
            let mut cum = 0.0f64;
            let mut k_in_block = block_amps - 1;
            let mut found = false;
            if let Some(ref td) = tape_dir_buf {
                let path = alloc::format!("{}/block_{:08}.bin", td, bi);
                if let Ok(bytes) = std::fs::read(&path) {
                    let n_cells = bytes.len() / core::mem::size_of::<f64>();
                    for i in 0..n_cells {
                        let off = i * core::mem::size_of::<f64>();
                        let n2 = f64::from_le_bytes([
                            bytes[off], bytes[off+1], bytes[off+2], bytes[off+3],
                            bytes[off+4], bytes[off+5], bytes[off+6], bytes[off+7],
                        ]);
                        cum += n2;
                        if local_target < cum { k_in_block = i; found = true; break; }
                    }
                }
            }
            if !found {
                let end = start + block_amps;
                let mut buf: Vec<Cx> = alloc::vec![Cx::zero(); block_amps];
                if !prepare_on_gpu {
                    let mut x = start;
                    while x < end { buf[x - start] = Cx::new(amp, 0.0); x += r_true_us; }
                }
                transform(start, m, &mut buf)?;
                cum = 0.0;
                for (i, c) in buf.iter().enumerate() {
                    cum += c.norm2();
                    if local_target < cum { k_in_block = i; break; }
                }
            }
            let k = start + k_in_block;
            if k == 0 {
                trace.push_str(&format!("  shot {}: k=0 — winding carries no information (s=0); re-measuring\n", total_shots));
                continue;
            }
            trace.push_str(&format!(
                "  shot {}: measured k={}  winding k/M = {}/{} of a full turn\n",
                total_shots, k, k, m));
            match attempt_big(k, m, &a, &n_val) {
                Attempt::Factors { r, s: _s, r0, p, q: f2, via } => {
                    trace.push_str(&format!(
                        "  continued fractions: k/M -> s/{} -> certified period r={}  ({} )\n",
                        r0, r, via));
                    return Ok(PhaseUnbraidResult {
                        tape_dir: _tape_dir_init_keepalive.clone(), n_val: n_val.clone(), a_used: a.clone(), n_qubits: q, m,
                        total_shots, shot_k: Some(k), certified_r: Some(r),
                        true_r: r_true.clone(), factors: Some((p, f2)), trace,
                    });
                }
                Attempt::Degenerate(msg) => {
                    trace.push_str(&format!("  {}\n", msg));
                    break;
                }
                Attempt::NoReadout => {
                    trace.push_str(&format!(
                        "  shot {}: no convergent of {}/{} certified a period; re-measuring\n",
                        total_shots, k, m));
                }
            }
        }
        a += BigUint::one();
    }
    Ok(PhaseUnbraidResult {
        tape_dir: _tape_dir_init_keepalive.clone(), n_val, a_used: a, n_qubits: q, m, total_shots, shot_k: None,
        certified_r: None, true_r: BigUint::zero(), factors: None, trace,
    })
}

/// Run the phase unbraider with dynamically sized inputs and the standard
/// in-memory amplitude budget. Generic `Into<BigUint>` inputs preserve callers
/// that already hold machine-sized values without imposing a machine-width
/// limit on callers that provide `BigUint`.
pub fn run_phase_unbraid<N, A>(
    n_val: N,
    a0: A,
    max_shots: u32,
) -> Result<PhaseUnbraidResult, String>
where
    N: Into<BigUint>,
    A: Into<BigUint>,
{
    run_phase_unbraid_big(n_val.into(), a0.into(), max_shots, 4_194_304, None)
}

/// The report: phase readout, closure, and the factors AS WORDS,
/// verified. Input is a decimal string (any length).
pub fn phase_unbraid_report_big(n_str: &str, a0: u64, max_shots: u32, mem_cap: usize, tape_dir: Option<String>) -> Result<String, String> {
    phase_unbraid_report_big_with_qft(n_str, a0, max_shots, mem_cap, tape_dir, false, |_, _, buffer| {
        qft(buffer);
        Ok(())
    })
}

pub fn phase_unbraid_report_big_with_qft<A, F>(n_str: &str, a0: A, max_shots: u32, mem_cap: usize, tape_dir: Option<String>, prepare_on_gpu: bool, transform: F) -> Result<String, String>
where
    A: Into<BigUint>,
    F: FnMut(usize, usize, &mut [Cx]) -> Result<(), String>,
{
    let n_val: BigUint = n_str.trim().parse::<BigUint>()
        .map_err(|_| format!("'{}' is not a decimal integer", n_str))?;
    let a0b = a0.into();
    let res = run_phase_unbraid_big_with_qft(n_val.clone(), a0b, max_shots, mem_cap, tape_dir, prepare_on_gpu, transform)?;
    let mut o = String::new();
    o.push_str("phase_unbraid (BigUint) — factors from a phase readout, no search\n");
    if let Some(ref td) = res.tape_dir {
        o.push_str(&format!("tape: {} (per-block amplitude norms spilled to disk; peak RAM = one block)\n", td));
    }
    o.push_str(&format!("N = {} ({} bits)\n", n_val, n_val.bits()));
    o.push_str(&format!("word: {}\n", native_numeral::encode(n_str.trim())));
    if res.n_qubits == 0 {
        let (p, qq) = res.factors.clone().unwrap();
        o.push_str(&res.trace);
        o.push_str(&format!("factors: p = {}\nq = {}\n", p, qq));
        o.push_str(&format!("p × q = N: {}\n", &p * &qq == n_val));
        o.push_str(&format!("syzygy preserves: {}\n", native_numeral::syzygy_preserves(&n_val, &p, &qq)));
        o.push_str(&native_numeral::factor_words_line(&p, &qq));
        return Ok(o);
    }
    o.push_str(&format!(
        "register: {} index qubits, M = 2^{} = {} amplitudes; streaming radix-2 FFT in blocks of {} amps\n",
        res.n_qubits, res.n_qubits, res.m, mem_cap.min(res.m)));
    if prepare_on_gpu {
        o.push_str(&format!("basis a = {}  (CUDA modular-exponentiation branch preparation; period not precomputed)\n", res.a_used));
    } else {
        o.push_str(&format!("basis a = {}  (CPU reference period r = {} used to prepare the amplitude comb)\n", res.a_used, res.true_r));
    }
    o.push_str("measurement record:\n");
    o.push_str(&res.trace);
    match res.factors {
        Some((p, qq)) => {
            o.push_str("FACTORS (one phase measurement + one gcd — no enumeration):\n");
            o.push_str(&format!("p = {}\nq = {}\n", p, qq));
            o.push_str(&format!("p × q = N: {}\n", &p * &qq == n_val));
            o.push_str(&format!("syzygy preserves: {}\n", native_numeral::syzygy_preserves(&n_val, &p, &qq)));
            o.push_str(&native_numeral::factor_words_line(&p, &qq));
        }
        None => {
            o.push_str(&format!(
                "no nontrivial closure in {} measurement(s) — reported as measured, not guessed\n",
                res.total_shots));
        }
    }
    Ok(o)
}



/// REPL-shaped wrapper: `phase_unbraid <N> [a0] [max_shots] [mem_cap]`.
/// Used by G-mOMonadOS REPL and direct CLI.
pub fn repl_phase_unbraid(args: &[&str]) -> String {
    if args.is_empty() {
        return "usage: phase_unbraid <N> [a0=2] [max_shots=8] [mem_cap=4194304] [--tape <dir>]".into();
    }
    let mut tape_dir: Option<String> = None;
    let mut pos: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--tape" {
            if let Some(td) = args.get(i+1) { tape_dir = Some((*td).to_string()); i += 2; continue; }
        }
        pos.push(args[i]); i += 1;
    }
    let n_str = match pos.first() { Some(s) => *s, None => return "phase_unbraid: missing N".into() };
    let a0: u64 = pos.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let max_shots: u32 = pos.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let mem_cap: usize = pos.get(3).and_then(|s| s.parse().ok()).unwrap_or(1usize << 22);
    match phase_unbraid_report_big(n_str, a0, max_shots, mem_cap, tape_dir) {
        Ok(s) => s,
        Err(e) => format!("phase_unbraid error: {}", e),
    }
}

#[cfg(test)]
mod phase_tests_big {
    use super::*;
    #[test]
    fn phase_accumulator_binding_128_bit_semiprime() {
        let n = semiprime_128();
        assert!(n.bits() >= 128);
        let base = &n - BigUint::one();
        let mut accumulator = PhaseReadoutAccumulator::default();
        assert!(accumulator.close(&BigUint::zero(), 256, &base, &n).unwrap().is_none());
        // Seed only the accumulator state, not an invented phase sample.
        accumulator.denominator = n.clone();
        assert!(accumulator.close(&BigUint::zero(), 256, &base, &n).unwrap().is_none());
        assert_eq!(accumulator.denominator, n);
        let another_base = &n - BigUint::from(2u8);
        assert!(accumulator.close(&BigUint::zero(), 256, &another_base, &n).unwrap().is_none());
        assert!(accumulator.denominator.is_one());
        assert_eq!(accumulator.source_base, Some((n, another_base)));
    }
    fn semiprime_128() -> BigUint {
        BigUint::from(18_446_744_073_709_551_557u64) * BigUint::from(18_446_744_073_709_551_533u64)
    }
    #[test]
    fn recycled_born_rank_preserves_the_full_semiprime_mass_width() {
        let n = semiprime_128();
        let total = &n * &n + BigUint::one();
        let expected = &total - BigUint::one();
        // First draw equals total and must be rejected. The second draw
        // selects its final rank, beyond the old 64-bit quantile grid.
        let mut words = total.to_u64_digits().into_iter().chain(expected.to_u64_digits());
        let observed = phase_born_rank(&total, || words.next().expect("bounded rejection draw")).unwrap();
        assert_eq!(observed, expected);
        assert!(observed.bits() > 128);
    }
    #[test]
    fn recycled_batched_measurement_matches_materialized_unstructured_state() {
        let n = BigUint::from(16_925_480_323_643_806_501u64) * BigUint::from(17_526_877_587_580_975_651u64);
        assert_eq!(n.bits(), 128);
        let base = BigUint::from(2u8);
        let qubits = 2 * n.bits() + 8;
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        for batch in [1usize, 7, 64] {
            let mut reference = RecycledPhaseState::new(&n, &base, qubits, &format).unwrap();
            let mut streamed = RecycledPhaseState::new(&n, &base, qubits, &format).unwrap();
            let mut reference_rng = 12345;
            let mut streamed_rng = reference_rng;
            for _ in 0..8 {
                let (keys, low, high) = reference.branches(batch, &mut recycled_phase_mix_cpu).unwrap();
                let zero_mass = fixed_born_mass(&low);
                let one_mass = fixed_born_mass(&high);
                let bit = phase_born_rank(&(&zero_mass + &one_mass), || phase_random_word(&mut reference_rng)).unwrap() >= zero_mass;
                reference.commit(keys, if bit { high } else { low }, bit).unwrap();
                let observed = streamed.measure_next_batched(batch, &mut streamed_rng, &mut |low, high, feedback, format| {
                    assert!(low.len() <= batch && high.len() <= batch);
                    recycled_phase_mix_cpu(low, high, feedback, format)
                }).unwrap();
                assert_eq!(observed, (bit, zero_mass, one_mass));
                assert_eq!(streamed.target, reference.target);
                assert_eq!(streamed.readout, reference.readout);
                assert_eq!(streamed_rng, reference_rng);
                assert_eq!(streamed.peak_support, reference.peak_support);
            }
        }
    }
    #[test]
    fn recycled_measurements_match_every_bin_of_the_dense_joint_qft() {
        let n = semiprime_128();
        for base in [BigUint::from(2u8), &n - BigUint::one()] {
            let qubits = 6u64;
            let count = 1usize << qubits;
            let format = FixedPointFormat::for_modulus(&n).unwrap();
            let mut expected = alloc::vec![0.0; count];
            let mut residue = BigUint::one();
            let mut joint = alloc::collections::BTreeMap::new();
            for index in 0..count {
                let lane = joint.entry(residue.clone()).or_insert_with(|| alloc::vec![Cx::zero(); count]);
                lane[index].re = 1.0 / (count as f64).sqrt();
                residue = (residue * &base) % &n;
            }
            for lane in joint.values_mut() {
                qft(lane);
                for (mass, amplitude) in expected.iter_mut().zip(lane) { *mass += amplitude.norm2(); }
            }
            let mut observed_total = 0.0;
            for (frequency, expected_mass) in expected.into_iter().enumerate() {
                let mut state = RecycledPhaseState::new(&n, &base, qubits, &format).unwrap();
                let mut probability = 1.0;
                for output_bit in 0..qubits {
                    let (keys, low, high) = state.branches(3, &mut recycled_phase_mix_cpu).unwrap();
                    let zero_mass: f64 = low.iter().map(|cell| cell.decode(&format).norm2()).sum();
                    let one_mass: f64 = high.iter().map(|cell| cell.decode(&format).norm2()).sum();
                    let bit = frequency & (1usize << output_bit) != 0;
                    let selected_mass = if bit { one_mass } else { zero_mass };
                    probability *= selected_mass / (zero_mass + one_mass);
                    if selected_mass == 0.0 { break; }
                    state.commit(keys, if bit { high } else { low }, bit).unwrap();
                }
                assert!((probability - expected_mass).abs() < 1e-11,
                    "N={n}, a={base}, bin={frequency}: recycled={probability}, dense={expected_mass}");
                observed_total += probability;
            }
            assert!((observed_total - 1.0).abs() < 1e-11);
        }
    }

    #[test]
    fn phase_denominators_combine_across_shots_without_walking_multiples() {
        let n = semiprime_128();
        let base = BigUint::from(2u8);
        let qubits = 264;
        let m = BigUint::one() << qubits;
        let mut accumulated = PhaseReadoutAccumulator::default();
        assert_eq!(accumulated.close(&(&m / 2u8), qubits, &base, &n).unwrap(), None);
        assert_eq!(accumulated.close(&(&m / 3u8), qubits, &base, &n).unwrap(), None);
        assert_eq!(accumulated.denominator, BigUint::from(6u8));
        assert_eq!(close_wide_phase_readout(&BigUint::zero(), qubits, &base, &n).unwrap(), None);
        assert_eq!(close_wide_phase_readout(&(&m / 2u8), qubits, &base, &n).unwrap(), None);
    }

    #[test]
    fn recycled_phase_reads_the_complete_register_above_host_index_width() {
        let n = semiprime_128();
        // Register execution control only; this order-two base cannot split N.
        let base = &n - BigUint::one();
        let qubits = n.bits() * 2 + 8;
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        let mut rng = 12345;
        let mut state = RecycledPhaseState::new(&n, &base, qubits, &format).unwrap();
        let trace = measure_recycled_phase(&mut state, 7, &mut rng, recycled_phase_mix_cpu).unwrap();
        assert_eq!(trace.lines().count(), qubits as usize + 1);
        assert!(state.readout.is_zero() || state.readout == (BigUint::one() << (qubits - 1) as usize));
        assert!(state.peak_support <= 2);
        assert_eq!(close_wide_phase_readout(&state.readout, qubits, &base, &n).unwrap(), None);
    }
    #[test]
    fn fixed_qft_twiddles_use_the_requested_precision() {
        let modulus = (BigUint::one() << 80usize) + BigUint::one();
        let format = FixedPointFormat::for_modulus(&modulus).unwrap();
        let quarter_turn = FixedComplex::qft_twiddle(&BigUint::one(), 1, false, &format).unwrap();
        let scale = format.scale();
        assert!(quarter_turn.re.abs() <= BigInt::from(8u8));
        assert!((&quarter_turn.im + &scale).abs() <= BigInt::from(8u8));
        let inverse = FixedComplex::qft_twiddle(&BigUint::one(), 1, true, &format).unwrap();
        assert_eq!(inverse.re, quarter_turn.re);
        assert_eq!(inverse.im, -quarter_turn.im);
        let eighth_turn = FixedComplex::qft_twiddle(&BigUint::one(), 2, false, &format).unwrap();
        let unit_norm_error = (&eighth_turn.re * &eighth_turn.re * BigInt::from(2u8) - &scale * &scale).abs();
        assert!(unit_norm_error <= &scale * BigInt::from(32u8));
        let f64_control = FixedComplex::from_f64(
            libm::cos(-core::f64::consts::PI / 4.0),
            libm::sin(-core::f64::consts::PI / 4.0),
            &format,
        ).unwrap();
        assert!((&eighth_turn.re - f64_control.re).abs() > (BigInt::one() << 100usize));
    }

    #[test]
    fn fixed_point_w_scales_with_modulus_and_retains_tiny_register_values() {
        let modulus = BigUint::one() << 9_999usize;
        let format = FixedPointFormat::for_modulus(&modulus).unwrap();
        assert_eq!(format.modulus_bits, 9_999);
        assert_eq!(format.w_bits, 20_014);
        let value = FixedComplex::from_f64(1.0, -0.5, &format).unwrap();
        assert_eq!(value.re, format.scale());
        assert_eq!(format.decode_f64(&value.re), 1.0);
        assert_eq!(format.decode_f64(&value.im), -0.5);
        let half = value.half();
        assert_eq!(format.decode_f64(&half.re), 0.5);
        assert_eq!(format.decode_f64(&half.im), -0.25);
    }

    #[test]
    fn dense_register_tape_roundtrips_a_biguint_addressed_tile() {
        let root = std::env::temp_dir().join(format!("dense-register-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let tape = DenseRegisterTape::create(root.to_string_lossy().into_owned(), DenseRegisterLayout::new(1_000, 4).unwrap()).unwrap();
        let tile_index = BigUint::one() << 900usize;
        let amplitudes = [Cx::new(1.25, -2.5), Cx::new(3.0, 4.0), Cx::zero(), Cx::new(-7.0, 0.5)];
        tape.write_tile(&tile_index, &amplitudes).unwrap();
        assert_eq!(tape.read_tile(&tile_index).unwrap().iter().map(|cell| (cell.re, cell.im)).collect::<Vec<_>>(), amplitudes.iter().map(|cell| (cell.re, cell.im)).collect::<Vec<_>>());
        let untouched = tape.read_tile(&(tile_index + BigUint::one())).unwrap();
        assert!(untouched.iter().all(|cell| cell.re == 0.0 && cell.im == 0.0));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixed_tape_roundtrips_a_biguint_tile() {
        let root = std::env::temp_dir().join(format!("fixed-register-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let format = FixedPointFormat::for_modulus(&BigUint::from(15u8)).unwrap();
        let tape = DenseRegisterTapeFixed::create(root.to_string_lossy().into_owned(), DenseRegisterLayout::new(1_000, 2).unwrap(), format.clone()).unwrap();
        let tile_index = BigUint::one() << 900usize;
        let amplitudes = [
            FixedComplex::from_f64(0.75, -0.5, &format).unwrap(),
            FixedComplex::from_f64(-0.875, 0.25, &format).unwrap(),
        ];
        tape.write_tile(&tile_index, &amplitudes).unwrap();
        assert_eq!(tape.read_tile(&tile_index).unwrap(), amplitudes);
        assert_eq!(tape.read_tile(&(tile_index + BigUint::one())).unwrap(), vec![FixedComplex { re: BigInt::zero(), im: BigInt::zero() }; 2]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixed_tape_roundtrips_three_limb_cells() {
        let root = std::env::temp_dir().join(format!("fixed-register-wide-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let modulus = (BigUint::one() << 80usize) + BigUint::one();
        let format = FixedPointFormat::for_modulus(&modulus).unwrap();
        assert_eq!(format.w_bits, 176);
        let tape = DenseRegisterTapeFixed::create(root.to_string_lossy().into_owned(), DenseRegisterLayout::new(200, 2).unwrap(), format.clone()).unwrap();
        let tile_index = BigUint::one() << 150usize;
        let amplitudes = [
            FixedComplex { re: -(BigInt::one() << 175usize) + BigInt::from(37u8), im: (BigInt::one() << 174usize) - BigInt::from(91u8) },
            FixedComplex { re: BigInt::one() << 160usize, im: -BigInt::one() },
        ];
        tape.write_tile(&tile_index, &amplitudes).unwrap();
        assert_eq!(tape.read_tile(&tile_index).unwrap(), amplitudes);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dense_register_tiles_use_biguint_global_addresses() {
        let layout = DenseRegisterLayout::new(1_000_000, 256).unwrap();
        let high_start = BigUint::one() << 999_999usize;
        assert!(layout.contains_range(&high_start, 256));
        assert!(!layout.contains_range(&(BigUint::one() << 1_000_000usize), 1));
        let (tile_start, count) = layout.tile(&BigUint::from(17u32)).unwrap();
        assert_eq!(tile_start, BigUint::from(4352u32));
        assert_eq!(count, 256);

        let wide = DenseRegisterLayout::new(1_000, 4).unwrap();
        let tile = BigUint::one() << 100usize;
        assert_eq!(wide.butterfly_partner_tile(&tile, 900), Some(&tile ^ (BigUint::one() << 898usize)));
        assert_eq!(wide.butterfly_partner_tile(&tile, 1), Some(tile.clone()));
    }

    #[test]
    fn dense_register_tape_lists_only_sorted_written_biguint_tiles() {
        let root = std::env::temp_dir().join(format!("sparse-register-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let tape = DenseRegisterTape::create(root.to_string_lossy().into_owned(), DenseRegisterLayout::new(1_000, 2).unwrap()).unwrap();
        let indices = [BigUint::one() << 900usize, BigUint::one() << 500usize, BigUint::from(3u8)];
        for index in &indices { tape.write_tile(index, &[Cx::zero(), Cx::zero()]).unwrap(); }
        let mut expected = indices.to_vec();
        expected.sort();
        assert_eq!(tape.written_tiles().unwrap(), expected);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fifteen_factors_by_phase() {
        let res = run_phase_unbraid_big(BigUint::from(15u32), BigUint::from(7u32), 12, 1<<16, None).unwrap();
        assert_eq!(res.factors, Some((BigUint::from(3u32), BigUint::from(5u32))));
        assert_eq!(res.certified_r, Some(BigUint::from(4u32)));
    }

    #[test]
    fn compatibility_entry_point_keeps_biguint_inputs() {
        let res = run_phase_unbraid(BigUint::from(15u32), BigUint::from(7u32), 12).unwrap();
        assert_eq!(res.factors, Some((BigUint::from(3u32), BigUint::from(5u32))));
    }
    #[test]
    fn sixtyfive_factors_by_phase() {
        let res = run_phase_unbraid_big(BigUint::from(65u32), BigUint::from(2u32), 16, 1<<16, None).unwrap();
        let (p, q) = res.factors.expect("65 must factor by phase readout");
        let n65 = BigUint::from(65u32);
        assert!(&p * &q == n65);
        assert!(p == BigUint::from(13u32) || p == BigUint::from(5u32));
    }
}
