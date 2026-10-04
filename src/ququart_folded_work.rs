//! Full ququart control coupled to shared work-register decision branches.
//! Modular operations execute reversible gates; no residue table or period is supplied.
use crate::anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::anyon_ququart::{FixedQuquartSic, QuquartDigit, QuquartSicOutcome};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::ququart_decision::DecisionArena;
use crate::ququart_factor::{emit_controlled_ququart_power, QuquartPhaseDevice};
use crate::reversible_modular::{ElementaryGate, ModularMultiply};
use alloc::{string::String, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};

pub struct QuquartFoldedWorkDevice {
    source: BigUint,
    format: FixedPointFormat,
    fourier: PairMatrix,
    sic: FixedQuquartSic,
    arena: DecisionArena,
    roots: [usize; 5],
    random: u64,
    active: bool,
    measured: Option<QuquartDigit>,
    expected: usize,
    count: usize,
    pub peak_nodes: usize,
}
impl QuquartFoldedWorkDevice {
    pub fn new(source: BigUint, fourier: PairMatrix, seed: u64) -> Result<Self, String> {
        let format = FixedPointFormat::for_modulus(&source)?;
        let sic = FixedQuquartSic::new(&format)?;
        let cells = ModularMultiply::new(&source)?.elementary_qubits() - 1;
        let mut arena = DecisionArena::new(cells);
        let zero = arena.zero();
        Ok(Self {
            format,
            source,
            fourier,
            sic,
            arena,
            roots: [zero; 5],
            random: seed.max(1),
            active: false,
            measured: None,
            expected: 0,
            count: 0,
            peak_nodes: 0,
        })
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
        if self.active && self.measured.is_none() {
            Ok(())
        } else {
            Err("gate outside an active unmeasured ququart stage".into())
        }
    }
    fn fold(&mut self) {
        self.arena.fold(&mut self.roots);
        self.peak_nodes = self.peak_nodes.max(self.arena.retained_nodes());
    }
    fn apply_gate(&mut self, gate: ElementaryGate) -> Result<(), String> {
        let (target, controls): (_, Vec<_>) = match gate {
            ElementaryGate::X(t) => (t, Vec::new()),
            ElementaryGate::Cnot { control, target } => (target, alloc::vec![control]),
            ElementaryGate::Toffoli {
                first,
                second,
                target,
            } => (target, alloc::vec![first, second]),
        };
        if target >= self.arena.cells + 2
            || controls
                .iter()
                .any(|&q| q >= self.arena.cells + 2 || q == target)
        {
            return Err("reversible gate has invalid ququart work wires".into());
        }
        let mut work: Vec<_> = controls
            .iter()
            .filter(|&&q| q >= 2)
            .map(|&q| q - 2)
            .collect();
        work.sort_unstable();
        let old = self.roots;
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            if controls.iter().any(|&q| q < 2 && digit & (1 << q) == 0) {
                continue;
            }
            self.roots[channel] = if target < 2 {
                let changed = old[COMPUTATIONAL_CHANNELS[digit ^ (1 << target)]];
                self.arena.conditional(old[channel], changed, &work)
            } else {
                self.arena.controlled_flip(old[channel], target - 2, &work)
            };
        }
        Ok(())
    }

    pub fn measure_sic_outcome(&mut self) -> Result<QuquartSicOutcome, String> {
        self.require_active()?;
        if self.count >= self.expected {
            return Err("ququart phase has extra SIC measurements".into());
        }
        let old = self.roots;
        let zero = self.arena.zero();
        let mut branches = [zero; 16];
        let mut masses: [BigUint; 17] = core::array::from_fn(|_| BigUint::zero());
        for (outcome, ray) in self.sic.rays().iter().enumerate() {
            let mut branch = zero;
            for (component, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let scalar = crate::phase_unbraid::FixedComplex {
                    re: ray[component].re.clone(),
                    im: -&ray[component].im,
                };
                let projected = self.arena.scale(old[channel], &scalar, &self.format);
                branch = self.arena.sum(branch, projected);
            }
            masses[outcome] = self.arena.mass(branch);
            branches[outcome] = branch;
        }
        masses[16] = self.arena.mass(old[LEAKAGE_CHANNEL]) * 4u8;
        let outcome = crate::anyon_fusion_kernel::sample_born_masses(&masses, |bytes| {
            Self::entropy(&mut self.random, bytes)
        })?;
        if outcome == 16 {
            let z = self.arena.zero();
            self.roots = [z; 5];
            self.fold();
            self.count += 1;
            return Ok(QuquartSicOutcome::OutsideCarrier);
        }
        let norm = BigInt::from(masses[outcome].sqrt());
        if norm.is_zero() {
            return Err("selected SIC outcome has zero Born mass".into());
        }
        let normalized = self.arena.normalize(branches[outcome], &norm, &self.format.scale());
        let z = self.arena.zero();
        self.roots = [z; 5];
        self.roots[COMPUTATIONAL_CHANNELS[0]] = normalized;
        self.fold();
        self.count += 1;
        Ok(QuquartSicOutcome::Carrier(
            crate::sic::SixteenOutcome::new(outcome as u8).map_err(|error| error.to_string())?,
        ))
    }
}
impl QuquartPhaseDevice for QuquartFoldedWorkDevice {
    fn measure_sic_outcome(&mut self) -> Result<QuquartSicOutcome, String> {
        QuquartFoldedWorkDevice::measure_sic_outcome(self)
    }

    fn begin(&mut self, source: &BigUint, _base: &BigUint, digits: usize) -> Result<(), String> {
        if self.active || source != &self.source || digits == 0 {
            return Err("ququart source differs from prepared register".into());
        }
        let register = crate::anyon_fusion_kernel::PreparedRegister::uniform(source, |bytes| {
            Self::entropy(&mut self.random, bytes)
        })?;
        self.arena = DecisionArena::new(self.arena.cells);
        let z = self.arena.zero();
        self.roots = [z; 5];
        self.roots[COMPUTATIONAL_CHANNELS[0]] =
            self.arena.basis(register.residue(), self.format.scale());
        self.active = true;
        self.measured = None;
        self.expected = digits;
        self.count = 0;
        self.fold();
        Ok(())
    }
    fn fourier(&mut self, inverse: bool) -> Result<(), String> {
        self.require_active()?;
        let matrix = if inverse {
            self.fourier.adjoint()
        } else {
            self.fourier.clone()
        };
        let old = self.roots;
        for row in 0..5 {
            let mut sum = self.arena.zero();
            for (col, &root) in old.iter().enumerate() {
                let term = self
                    .arena
                    .scale(root, &matrix.0[5 * row + col], &self.format);
                sum = self.arena.sum(sum, term);
            }
            self.roots[row] = sum;
        }
        self.fold();
        Ok(())
    }
    fn controlled_multiply(&mut self, multiplier: &BigUint) -> Result<(), String> {
        self.require_active()?;
        let source = self.source.clone();
        emit_controlled_ququart_power(&source, multiplier, |gate| self.apply_gate(gate))?;
        self.fold();
        Ok(())
    }
    fn feedback(&mut self, numerator: &BigUint, digits: usize) -> Result<(), String> {
        self.require_active()?;
        let bits = digits.checked_mul(2).ok_or("phase width overflow")?;
        let denominator = BigUint::one() << bits;
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let phase = FixedComplex::winding_twiddle(
                &-BigInt::from(numerator * digit),
                &denominator,
                &self.format,
            )?;
            self.roots[channel] = self.arena.scale(self.roots[channel], &phase, &self.format);
        }
        self.fold();
        Ok(())
    }
    fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
        self.require_active()?;
        if self.count >= self.expected {
            return Err("ququart phase has extra measurements".into());
        }
        let masses: [BigUint; 5] = core::array::from_fn(|digit| {
            self.arena.mass(
                self.roots[if digit < 4 {
                    COMPUTATIONAL_CHANNELS[digit]
                } else {
                    LEAKAGE_CHANNEL
                }],
            )
        });
        let outcome = crate::anyon_fusion_kernel::sample_born_masses(&masses, |bytes| {
            Self::entropy(&mut self.random, bytes)
        })?;
        if outcome == 4 {
            return Err("ququart measurement reached the outside-carrier fusion channel".into());
        }
        let selected = COMPUTATIONAL_CHANNELS[outcome];
        let norm = BigInt::from(masses[outcome].sqrt());
        if norm.is_zero() {
            return Err("selected ququart has zero Born mass".into());
        }
        let root = self
            .arena
            .normalize(self.roots[selected], &norm, &self.format.scale());
        let z = self.arena.zero();
        self.roots = [z; 5];
        self.roots[selected] = root;
        let digit = QuquartDigit::try_from(outcome as u8)?;
        self.measured = Some(digit);
        self.count += 1;
        self.fold();
        Ok(digit)
    }
    fn reset_control(&mut self, digit: QuquartDigit) -> Result<(), String> {
        if !self.active || self.measured != Some(digit) {
            return Err("ququart reset differs from its measurement".into());
        }
        let root = self.roots[COMPUTATIONAL_CHANNELS[digit as usize]];
        let z = self.arena.zero();
        self.roots = [z; 5];
        self.roots[COMPUTATIONAL_CHANNELS[0]] = root;
        self.measured = None;
        self.fold();
        Ok(())
    }
    fn finish(&mut self) -> Result<(), String> {
        if !self.active || self.measured.is_some() || self.count != self.expected {
            return Err("ququart phase measurements are incomplete".into());
        }
        self.abort();
        Ok(())
    }
    fn abort(&mut self) {
        self.arena = DecisionArena::new(self.arena.cells);
        let z = self.arena.zero();
        self.roots = [z; 5];
        self.active = false;
        self.measured = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_work_sic_measurement_preserves_mask_and_work_closure() {
        let n = BigUint::parse_bytes(
            b"287032184953460072652382545924346253693",
            10,
        )
        .unwrap();
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        let mut device = QuquartFoldedWorkDevice::new(
            n.clone(),
            PairMatrix::identity(&format),
            1729,
        )
        .unwrap();
        device.begin(&n, &BigUint::from(2u8), 1).unwrap();
        let outcome = device.measure_sic_outcome().unwrap();
        assert!(matches!(outcome, QuquartSicOutcome::Carrier(state) if state.mask() < 16));
        let selected = device.roots[COMPUTATIONAL_CHANNELS[0]];
        assert_eq!(device.arena.mass(selected), format.scale().to_biguint().unwrap().pow(2));
        for &channel in &COMPUTATIONAL_CHANNELS[1..] {
            assert!(device.arena.mass(device.roots[channel]).is_zero());
        }
        assert!(device.arena.mass(device.roots[LEAKAGE_CHANNEL]).is_zero());
    }

    #[test]
    fn shared_work_gates_preserve_all_four_control_correlations() {
        let n = BigUint::parse_bytes(
            b"74190557381655886253556359637698917958655348096397072330011641798610025580603",
            10,
        )
        .unwrap();
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        let mut matrix = PairMatrix(core::array::from_fn(|_| FixedComplex {
            re: BigInt::zero(),
            im: BigInt::zero(),
        }));
        for (row, &r) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (col, &c) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let mut z = FixedComplex::winding_twiddle(
                    &BigInt::from(row * col),
                    &BigUint::from(4u8),
                    &format,
                )
                .unwrap();
                z.re /= 2u8;
                z.im /= 2u8;
                matrix.0[5 * r + c] = z;
            }
        }
        matrix.0[5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL].re = format.scale();
        let mut device = QuquartFoldedWorkDevice::new(n.clone(), matrix, 1729).unwrap();
        device.begin(&n, &BigUint::from(2u8), 260).unwrap();
        let mut replay_seed = 1729u64;
        let register = crate::anyon_fusion_kernel::PreparedRegister::uniform(&n, |bytes| {
            QuquartFoldedWorkDevice::entropy(&mut replay_seed, bytes)
        })
        .unwrap();
        // Capture the sampled basis from an independent replay of the seed.
        let x = register.residue();
        device.fourier(false).unwrap();
        device
            .apply_gate(ElementaryGate::Cnot {
                control: 0,
                target: 2,
            })
            .unwrap();
        device
            .apply_gate(ElementaryGate::Cnot {
                control: 1,
                target: 4,
            })
            .unwrap();
        device.apply_gate(ElementaryGate::Toffoli { first: 1, second: 2, target: 5 }).unwrap();
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let mut image = x.clone();
            if digit & 1 != 0 {
                image ^= BigUint::one();
            }
            if digit & 2 != 0 {
                image ^= BigUint::one() << 2usize;
            }
            if digit & 2 != 0 && image.bit(0) { image ^= BigUint::one() << 3usize; }
            let (re, im) = device.arena.amplitude(device.roots[channel], &image);
            assert_eq!(re, format.scale() / 2u8);
            assert!(im.is_zero());
            assert_eq!(
                device.arena.mass(device.roots[channel]),
                (format.scale() / 2u8).pow(2).to_biguint().unwrap()
            );
        }
        device.apply_gate(ElementaryGate::Toffoli { first: 1, second: 2, target: 5 }).unwrap();
        device
            .apply_gate(ElementaryGate::Cnot {
                control: 1,
                target: 4,
            })
            .unwrap();
        device
            .apply_gate(ElementaryGate::Cnot {
                control: 0,
                target: 2,
            })
            .unwrap();
        device.fourier(true).unwrap();
        let (re, im) = device
            .arena
            .amplitude(device.roots[COMPUTATIONAL_CHANNELS[0]], x);
        assert_eq!(re, format.scale());
        assert!(im.is_zero());
        for &channel in &COMPUTATIONAL_CHANNELS[1..] {
            assert!(device.arena.mass(device.roots[channel]).is_zero());
        }
    }
}
