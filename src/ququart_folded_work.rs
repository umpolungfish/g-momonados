//! Full ququart control coupled to shared work-register decision branches.
//! Modular operations execute reversible gates; no residue table or period is supplied.
use crate::anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::anyon_ququart::{FixedQuquartSic, QuquartDigit, QuquartSicOutcome};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::ququart_decision::DecisionArena;
use crate::ququart_factor::QuquartPhaseDevice;
use crate::reversible_modular::{NestedOperation, ModularMultiply};
#[cfg(test)]
use crate::reversible_modular::ElementaryGate;
use alloc::{string::String, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use core::sync::atomic::{AtomicU64, Ordering};

// Read by Vox while the native process is stopped. These counters observe the
// resident execution; they neither supply work values nor emit a readout.
#[no_mangle]
pub static VOX_QUQUART_COUNTERS: [AtomicU64; 8] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];

pub struct PreparedModularWork {
    pub source: BigUint,
    pub digit_bits: usize,
    pub operations: Vec<NestedOperation>,
    pub stages: Vec<(BigUint,Vec<usize>)>,
}
impl PreparedModularWork {
    /// Recover every source-bound operation at every phase-stack stage. This
    /// checks the executable map, not just the shape of the serialized pool.
    pub fn verify_recovery(&self, source: &BigUint, base: &BigUint, digit_bits: usize, digits: usize)
        -> Result<(),String>
    {
        if self.source != *source || self.digit_bits != digit_bits || digit_bits == 0 {
            return Err("prepared work source or radix differs from the resident register".into());
        }
        let schedule = crate::ququart_factor::QuquartPowerSchedule::prepare(source,base)?;
        if self.stages.len() != digits || schedule.powers().len() != digits {
            return Err("prepared work has incomplete stack height".into());
        }
        let arithmetic = ModularMultiply::new(source)?;
        for (height,((multiplier,references),expected)) in self.stages.iter().zip(schedule.powers()).enumerate() {
            if multiplier != expected {
                return Err(alloc::format!("prepared work multiplier changed at stack height {}",height));
            }
            let mut position = 0usize;
            for (lane,power) in [(0,multiplier.clone()),(1,multiplier*multiplier%source)] {
                arithmetic.emit_ququart_nested_radix(&power,lane,digit_bits,|operation| {
                    let observed = references.get(position).and_then(|&id|self.operations.get(id));
                    if observed != Some(&operation) {
                        return Err(alloc::format!("prepared work recovery changed operation {} at stack height {}",position,height));
                    }
                    position += 1;
                    Ok(())
                })?;
            }
            if position != references.len() {
                return Err(alloc::format!("prepared work has an extra operation at stack height {}",height));
            }
        }
        Ok(())
    }
}

pub struct SicControlWitness {
    pub gram: [(BigInt,BigInt);16],
    pub masses: [BigUint;16],
    pub digit: QuquartDigit,
}
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
    interleaved_work: bool,
    digit_bits: usize,
    prepared_work: Option<PreparedModularWork>,
    sic_witnesses: Vec<SicControlWitness>,
    nested_since_fold: usize,
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
            interleaved_work: false,
            digit_bits: 1,
            prepared_work: None,
            sic_witnesses: Vec::new(),
            nested_since_fold: 0,
            peak_nodes: 0,
        })
    }
    pub fn with_prepared_work(mut self, work: PreparedModularWork) -> Self {
        self.prepared_work = Some(work);
        self
    }
    pub fn sic_witnesses(&self) -> &[SicControlWitness] { &self.sic_witnesses }
    pub fn new_interleaved(source: BigUint, fourier: PairMatrix, seed: u64) -> Result<Self, String> {
        let mut device = Self::new(source, fourier, seed)?;
        device.interleaved_work = true;
        Ok(device)
    }
    pub fn new_interleaved_radix(source: BigUint, fourier: PairMatrix, seed: u64, radix_word: &str) -> Result<Self,String> {
        let radix = crate::ququart_factor::power_of_two_radix_word(radix_word)?;
        let mut device = Self::new_interleaved(source,fourier,seed)?;
        device.digit_bits = radix.bits_le().len()-1;
        Ok(device)
    }
    fn work_wire(&self, wire: usize) -> usize {
        let width = self.source.bits() as usize;
        if !self.interleaved_work || wire < 2 || wire > 2 * width + 1 { return wire; }
        if wire <= width + 1 { 2 * (wire - 1) }
        else { 2 * (wire - width - 1) + 1 }
    }
    fn work_address(&self, residue: &BigUint) -> BigUint {
        if !self.interleaved_work { return residue.clone(); }
        let mut address = BigUint::zero();
        for bit in 0..residue.bits() {
            address.set_bit(2 * bit, residue.bit(bit));
        }
        address
    }
    fn require_clean_workspace(&self) -> Result<(),String> {
        let width = self.source.bits() as usize;
        for (channel,&root) in self.roots.iter().enumerate() {
            // Check the whole allocated stack: work accumulator, borrow flag,
            // ripple carries, and control-lowering ancilla. The fifth fusion
            // channel carries workspace too, even outside the computation.
            for logical in width+2..self.arena.cells+2 {
                let wire = self.work_wire(logical)-2;
                if !self.arena.wire_is_zero(root,wire) {
                    let word = |value:usize| crate::godel_calculus::encode_cell_binary(
                        &crate::godel_calculus::Nat::from_bits_le(
                            (0..usize::BITS).map(|bit| value & (1usize << bit) != 0).collect()));
                    return Err(alloc::format!("modular cleanup left dirty workspace: channel_word={} wire_word={}",word(channel),word(logical)));
                }
            }
        }
        Ok(())
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
        self.nested_since_fold = 0;
        self.peak_nodes = self.peak_nodes.max(self.arena.retained_nodes());
        VOX_QUQUART_COUNTERS[4].store(self.arena.retained_nodes() as u64, Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[5].store(self.peak_nodes as u64, Ordering::Relaxed);
    }
    #[cfg(test)]
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

    fn apply_nested(&mut self, operation: NestedOperation) -> Result<(), String> {
        VOX_QUQUART_COUNTERS[3].fetch_add(1, Ordering::Relaxed);
        let map_register = |r: Vec<usize>| r.into_iter().map(|wire| self.work_wire(wire)).collect();
        let map_controls = |c: Vec<(usize,bool)>| c.into_iter()
            .map(|(wire,value)| (self.work_wire(wire),value)).collect();
        let operation = match operation {
            NestedOperation::Toggle(gate) => NestedOperation::Toggle(crate::reversible_modular::ControlledX {
                target: self.work_wire(gate.target), controls: map_controls(gate.controls),
            }),
            NestedOperation::ModularAdd { register,digit,value,modulus,controls } => NestedOperation::ModularAdd {
                register: map_register(register),digit: map_register(digit),value,modulus,controls: map_controls(controls),
            },
            NestedOperation::Add { register,value,controls } => NestedOperation::Add {
                register: map_register(register),value,controls: map_controls(controls),
            },
            NestedOperation::Compare { register,value,controls,flag } => NestedOperation::Compare {
                register: map_register(register),value,controls: map_controls(controls),flag: self.work_wire(flag),
            },
        };
        let (controls, targets): (&[(usize,bool)], Vec<usize>) = match &operation {
            NestedOperation::Toggle(gate) => (&gate.controls, alloc::vec![gate.target]),
            NestedOperation::Add { register, controls, .. } | NestedOperation::ModularAdd { register, controls, .. } => (controls, register.clone()),
            NestedOperation::Compare { register, controls, flag, .. } => {
                if register.last().is_some_and(|wire| wire >= flag) {
                    return Err("nested comparison flag must follow its register".into());
                }
                (controls, alloc::vec![*flag])
            }
        };
        let extent = self.arena.cells + 2;
        if let NestedOperation::ModularAdd { register,digit,controls,.. } = &operation {
            if digit.is_empty() || digit.iter().any(|&wire| wire < 2 || wire >= extent || register.contains(&wire))
                || digit.windows(2).any(|pair| pair[0] >= pair[1])
                || controls.iter().any(|&(wire,_)| digit.contains(&wire)) {
                return Err("modular source digit requires ordered disjoint wires".into());
            }
        }
        if controls.iter().any(|&(wire,_)| wire >= extent || targets.contains(&wire))
            || targets.iter().any(|&wire| wire >= extent) {
            return Err("nested arithmetic has invalid or overlapping wires".into());
        }
        if let NestedOperation::Add { register, .. } | NestedOperation::Compare { register, .. } | NestedOperation::ModularAdd { register, .. } = &operation {
            if register.is_empty() || register[0] < 2
                || register.windows(2).any(|pair| pair[0] >= pair[1])
                || register.iter().any(|&wire| wire >= extent)
                || controls.iter().any(|&(wire,_)| register.contains(&wire)) {
                return Err("nested arithmetic requires ordered disjoint work wires".into());
            }
        }
        let mut work: Vec<_> = controls.iter().filter(|&&(wire,_)| wire >= 2)
            .map(|&(wire,value)| (wire-2,value)).collect();
        work.sort_unstable();
        if work.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err("nested arithmetic controls must address distinct wires".into());
        }
        let old = self.roots;
        for (digit,&channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let incoming = match &operation {
                NestedOperation::Toggle(gate) if gate.target < 2 =>
                    old[COMPUTATIONAL_CHANNELS[digit ^ (1 << gate.target)]],
                _ => old[channel],
            };
            if controls.iter().any(|&(wire,value)| wire < 2 && (digit & (1 << wire) != 0) != value)
                || (!self.arena.controls_possible(old[channel],&work)
                    && !self.arena.controls_possible(incoming,&work)) {
                continue;
            }
            self.roots[channel] = match &operation {
                NestedOperation::Toggle(gate) => {
                    let changed = if gate.target < 2 {
                        incoming
                    } else {
                        let zero = self.arena.zero();
                        let enabled = self.arena.conditional_literals(zero,old[channel],&work);
                        self.arena.flip(enabled,gate.target-2)
                    };
                    self.arena.conditional_literals(old[channel],changed,&work)
                }
                NestedOperation::ModularAdd { register,digit,value,modulus,.. } => self.arena.modular_digit_add(
                    old[channel],&register.iter().map(|wire| wire-2).collect::<Vec<_>>(),
                    &digit.iter().map(|wire| wire-2).collect::<Vec<_>>(),value,modulus,&work),
                NestedOperation::Add { register,value,.. } => self.arena.add_constant(
                    old[channel],&register.iter().map(|wire| wire-2).collect::<Vec<_>>(),value,&work),
                NestedOperation::Compare { register,value,flag,.. } => self.arena.compare_constant(
                    old[channel],&register.iter().map(|wire| wire-2).collect::<Vec<_>>(),value,&work,flag-2),
            };
        }
        self.nested_since_fold += 1;
        if self.nested_since_fold == 8 { self.fold(); }
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
        if let Some(work) = &self.prepared_work {
            work.verify_recovery(source,_base,self.digit_bits,digits)?;
        }
        VOX_QUQUART_COUNTERS[0].store(source.bits(), Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[1].store(digits as u64, Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[2].store(0, Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[3].store(0, Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[6].fetch_add(1, Ordering::Relaxed);
        let register = crate::anyon_fusion_kernel::PreparedRegister::uniform(source, |bytes| {
            Self::entropy(&mut self.random, bytes)
        })?;
        self.arena = DecisionArena::new(self.arena.cells);
        let z = self.arena.zero();
        self.roots = [z; 5];
        let address = self.work_address(register.residue());
        self.roots[COMPUTATIONAL_CHANNELS[0]] =
            self.arena.basis(&address, self.format.scale());
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
        if let Some(work) = self.prepared_work.take() {
            let result = match work.stages.iter().find(|(power,_)| power == multiplier) {
                Some((_,indices)) => indices.iter().try_for_each(|&index| {
                    let operation = work.operations.get(index).ok_or("prepared operation reference outside pool")?;
                    self.apply_nested(operation.clone())
                }),
                None => Err("controlled multiplier absent from baked work schedule".into()),
            };
            self.prepared_work = Some(work);
            result?;
            self.fold();
            self.require_clean_workspace()?;
            return Ok(());
        }
        let arithmetic = ModularMultiply::new(&self.source)?;
        arithmetic.emit_ququart_nested_radix(multiplier,0,self.digit_bits,|operation| self.apply_nested(operation))?;
        let square = multiplier * multiplier % &self.source;
        arithmetic.emit_ququart_nested_radix(&square,1,self.digit_bits,|operation| self.apply_nested(operation))?;
        self.fold();
        self.require_clean_workspace()?;
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
        let gram = self.arena.control_gram(&core::array::from_fn(|digit| self.roots[COMPUTATIONAL_CHANNELS[digit]]));
        let sic_masses = self.sic.gram_masses(&gram)?;
        self.sic.validate_gram_frame(&gram,&sic_masses)?;
        for digit in 0..4 {
            if gram[5*digit].0 != BigInt::from(masses[digit].clone()) {
                return Err("shared control Gram differs from computational Born mass".into());
            }
        }
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
        self.sic_witnesses.push(SicControlWitness { gram, masses:sic_masses, digit });
        self.measured = Some(digit);
        self.count += 1;
        VOX_QUQUART_COUNTERS[2].store(self.count as u64, Ordering::Relaxed);
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
        VOX_QUQUART_COUNTERS[7].fetch_add(1, Ordering::Relaxed);
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
    fn rsa_200_and_256_bit_prepared_work_is_checked_before_resident_entry() {
        let sources: Vec<_> = [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ].iter().map(|decimal|BigUint::parse_bytes(decimal.as_bytes(),10).unwrap()).collect();
        for (index,source) in sources.iter().enumerate() {
            assert!(source.bits() >= 200);
            let format = FixedPointFormat::for_modulus(source).unwrap();
            let base = BigUint::from(2u8);
            let schedule = crate::ququart_factor::QuquartPowerSchedule::prepare(source,&base).unwrap();
            for fault in 0..4 {
                let mut prepared = PreparedModularWork { source:source.clone(),digit_bits:1,
                    operations:Vec::new(),stages:schedule.powers().iter().map(|power|(power.clone(),Vec::new())).collect() };
                match fault {
                    0 => prepared.source = sources[1-index].clone(),
                    1 => prepared.digit_bits = 4,
                    2 => { prepared.stages.pop(); },
                    _ => {} // Correct metadata still cannot admit an empty executable map.
                }
                let mut device = QuquartFoldedWorkDevice::new_interleaved(
                    source.clone(),PairMatrix::identity(&format),1729).unwrap().with_prepared_work(prepared);
                let random = device.random;
                assert!(device.begin(source,&base,schedule.powers().len()).is_err());
                assert!(!device.active);
                assert_eq!(device.random,random,"invalid prepared work must not enter a measured state");
            }
        }
    }

    #[test]
    fn rsa_200_and_256_bit_workspace_check_reaches_the_top_of_every_channel() {
        for source in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let n = BigUint::parse_bytes(source.as_bytes(),10).unwrap();
            assert!(n.bits() >= 200);
            let format = FixedPointFormat::for_modulus(&n).unwrap();
            for interleaved in [false,true] {
                let mut device = QuquartFoldedWorkDevice::new(n.clone(),PairMatrix::identity(&format),1729).unwrap();
                device.interleaved_work = interleaved;
                let clean = device.arena.basis(&device.work_address(&(&n-1u8)),format.scale());
                for channel in 0..5 {
                    device.roots[channel] = clean;
                }
                assert!(device.require_clean_workspace().is_ok());
                // Exercise every workspace height, including the previously
                // unchecked borrow flag and the top control-lowering ancilla.
                for logical in n.bits() as usize+2..device.arena.cells+2 {
                    let dirty_wire = device.work_wire(logical)-2;
                    let dirty = device.arena.flip(clean,dirty_wire);
                    for channel in 0..5 {
                        device.roots[channel] = dirty;
                        assert!(device.require_clean_workspace().is_err(),
                            "upper workspace in channel {channel} was not checked");
                        device.roots[channel] = clean;
                    }
                }
                assert!(device.require_clean_workspace().is_ok());
            }
        }
    }

    #[test]
    fn rsa_200_and_256_bit_nested_modular_operator_preserves_complex_coherence() {
        for source in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
        let n = BigUint::parse_bytes(source.as_bytes(),10).unwrap();
        assert!(n.bits() >= 200);
        for interleaved in [false,true] {
        for digit_bits in [1,4] {
        let source_word = crate::godel_calculus::encode_cell_binary(
            &crate::godel_calculus::Nat::from_bits_le((0..n.bits()).map(|bit| n.bit(bit)).collect()));
        let reading = crate::godel_calculus::decode(source_word.trim()).unwrap();
        assert_eq!(crate::godel_calculus::encode_cell_binary(&reading.value),source_word.trim());
        let n = reading.value.bits_le().iter().rev().fold(BigUint::zero(),
            |value,&bit| (value << 1usize) + u8::from(bit));
        let format = FixedPointFormat::for_modulus(&n).unwrap();
        let mut device = QuquartFoldedWorkDevice::new(n.clone(),PairMatrix::identity(&format),1729).unwrap();
        device.interleaved_work = interleaved;
        device.digit_bits = digit_bits;
        device.begin(&n,&BigUint::from(2u8),1).unwrap();
        device.arena = DecisionArena::new(device.arena.cells);
        let z = device.arena.zero();
        device.roots = [z;5];
        let residues = [&n-1u8,&n-2u8];
        for (digit,&channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (index,residue) in residues.iter().enumerate() {
                let basis = device.arena.basis(&device.work_address(residue),format.scale());
                let scalar = FixedComplex { re: BigInt::from(3+5*digit+index), im: BigInt::from(7+3*digit+2*index) };
                let basis = device.arena.scale(basis,&scalar,&format);
                device.roots[channel] = device.arena.sum(device.roots[channel],basis);
            }
        }
        device.fold();
        let masses: Vec<_> = COMPUTATIONAL_CHANNELS.iter().map(|&c| device.arena.mass(device.roots[c])).collect();
        let schedule = crate::ququart_factor::QuquartPowerSchedule::prepare(&n,&BigUint::from(2u8)).unwrap();
        let multiplier = &schedule.powers()[64];
        device.controlled_multiply(multiplier).unwrap();
        let mut power = BigUint::one();
        for (digit,&channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (index,residue) in residues.iter().enumerate() {
                let image = residue*&power%&n;
                assert_eq!(device.arena.amplitude(device.roots[channel],&device.work_address(&image)),
                    (BigInt::from(3+5*digit+index),BigInt::from(7+3*digit+2*index)));
            }
            // Exact mass at the clean-work addresses exhausts the channel:
            // no amplitude can remain in any arithmetic or carry ancilla.
            assert_eq!(device.arena.mass(device.roots[channel]),masses[digit]);
            power = power*multiplier%&n;
        }
        assert!(device.arena.mass(device.roots[LEAKAGE_CHANNEL]).is_zero());
        println!("source_bits={} interleaved={} digit_bits={} control_channels={} complex_coherence_preserved=true workspace_clean=true",n.bits(),interleaved,digit_bits,COMPUTATIONAL_CHANNELS.len());
        }
        }
        }
    }

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
