//! Full ququart control coupled to shared work-register branches carried as
//! exact sparse basis records. Modular operations execute reversible gates;
//! no residue table or period is supplied. Every root is a deduplicated bag
//! of resolved work-basis values; the only superposition enters through the
//! five-root Fourier mixing, so each operation acts per branch with exact
//! fixed-point complex numerators instead of rebuilding decision diagrams.
use crate::anyon_pair::{PairMatrix, COMPUTATIONAL_CHANNELS, LEAKAGE_CHANNEL};
use crate::anyon_ququart::{FixedQuquartSic, QuquartDigit, QuquartSicOutcome};
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::ququart_factor::QuquartPhaseDevice;
use crate::reversible_modular::{NestedOperation, ModularMultiply};
#[cfg(test)]
use crate::reversible_modular::ElementaryGate;
use alloc::{string::String, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use crate::word_tape::{SignedWordTape, WordTape};
use core::sync::atomic::{AtomicU64, Ordering};

// Read by Vox while the native process is stopped. These counters observe the
// resident execution; they neither supply work values nor emit a readout.
#[no_mangle]
pub static VOX_QUQUART_COUNTERS: [AtomicU64; 8] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];

#[cfg_attr(not(test), no_mangle)]
pub static VOX_NESTED_OPERATION_COUNTERS: [AtomicU64; 4] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];

/// One retained basis branch: the complete work-register value over the arena
/// wires and its exact fixed-point complex numerator on that branch.
#[derive(Clone, PartialEq)]
struct WorkBranch {
    value: BigUint,
    re: SignedWordTape,
    im: SignedWordTape,
}

impl Default for WorkBranch {
    fn default() -> Self {
        Self { value: BigUint::default(), re: SignedWordTape::zero(), im: SignedWordTape::zero() }
    }
}

/// The executing ququart carrier. Every root is a bag of resolved basis
/// branches kept sorted and deduplicated by value; amplitudes add exactly and
/// complex scalars round per branch exactly as the diagram leaves did.
#[derive(Clone, Default, PartialEq)]
struct WorkBag(Vec<WorkBranch>);

impl WorkBag {
    fn zero() -> Self { Self(Vec::new()) }

    fn basis(value: &BigUint, scale: &BigInt) -> Self {
        WorkBag(alloc::vec![WorkBranch {
            value: value.clone(),
            re: SignedWordTape::from_bigint(scale),
            im: SignedWordTape::zero(),
        }])
    }

    /// Merge in branch order; equal values add their amplitudes and zero
    /// results drop, matching diagram summation at shared leaves.
    fn merge(mut self, mut other: WorkBag) -> Self {
        self.0.append(&mut other.0);
        self.0.sort_by(|a, b| a.value.cmp(&b.value));
        let mut out: Vec<WorkBranch> = Vec::with_capacity(self.0.len());
        for branch in self.0 {
            if branch.re.is_zero() && branch.im.is_zero() { continue; }
            match out.last_mut() {
                Some(last) if last.value == branch.value => {
                    last.re = last.re.add(&branch.re);
                    last.im = last.im.add(&branch.im);
                }
                _ => out.push(branch),
            }
        }
        out.retain(|b| !(b.re.is_zero() && b.im.is_zero()));
        WorkBag(out)
    }

    /// Drop branches whose amplitude became exactly zero. Per-branch maps
    /// preserve the sorted order, so only the zero leaves need clearing.
    fn deduped(self) -> Self {
        let mut out = self.0;
        out.retain(|b| !(b.re.is_zero() && b.im.is_zero()));
        WorkBag(out)
    }

    /// The common fixed-point complex scalar product used by the diagram
    /// scale: real = re*sr - im*si, imaginary = re*si + im*sr, rounded per
    /// branch with the same shift the leaves apply.
    fn scale(self, scalar: &FixedComplex, format: &FixedPointFormat) -> Self {
        if scalar.re.is_zero() && scalar.im.is_zero() { return Self::zero(); }
        let sr = SignedWordTape::from_bigint(&scalar.re);
        let si = SignedWordTape::from_bigint(&scalar.im);
        WorkBag(self.0.into_iter().map(|branch| WorkBranch {
            re: branch.re.mul(&sr).sub(&branch.im.mul(&si)).rounded_shr(format.w_bits as usize),
            im: branch.re.mul(&si).add(&branch.im.mul(&sr)).rounded_shr(format.w_bits as usize),
            value: branch.value,
        }).collect()).deduped()
    }

    fn mass(&self) -> BigUint {
        self.0.iter().map(|branch| {
            let re = branch.re.to_bigint();
            let im = branch.im.to_bigint();
            (&re * &re + &im * &im).to_biguint().unwrap()
        }).fold(BigUint::zero(), |a, b| a + b)
    }

    fn amplitude(&self, value: &BigUint) -> (BigInt, BigInt) {
        match self.0.binary_search_by(|branch| branch.value.cmp(value)) {
            Ok(index) => (self.0[index].re.to_bigint(), self.0[index].im.to_bigint()),
            Err(_) => (BigInt::zero(), BigInt::zero()),
        }
    }

    /// Whether at least one branch satisfies every literal. Wire arguments
    /// arrive already mapped to arena-bit coordinates.
    fn controls_satisfiable(&self, controls: &[(usize, bool)]) -> bool {
        self.0.iter().any(|branch| controls.iter().all(
            |&(wire, want)| branch.value.bit(wire as u64) == want))
    }

    fn wire_is_zero(&self, wire: usize) -> bool {
        self.0.iter().all(|branch| !branch.value.bit(wire as u64))
    }

    /// (branches matching every literal, the remaining branches).
    fn split_literals(&self, controls: &[(usize, bool)]) -> (Self, Self) {
        let mut matched = Vec::new();
        let mut rest = Vec::new();
        for branch in &self.0 {
            if controls.iter().all(
                |&(wire, want)| branch.value.bit(wire as u64) == want) {
                matched.push(branch.clone());
            } else {
                rest.push(branch.clone());
            }
        }
        (WorkBag(matched), WorkBag(rest))
    }

    /// Toggle one arena wire on every branch. Toggling is a value bijection,
    /// so the bag stays collision-free without a re-deduplication.
    fn toggle_bit(self, wire: usize) -> Self {
        WorkBag(self.0.into_iter().map(|mut branch| {
            branch.value ^= BigUint::one() << wire as u64;
            branch
        }).collect())
    }

    /// `(field + value) mod 2^field_len` on the register coordinates of every
    /// branch, the same wrapping ripple-carry addition the diagram emitted.
    fn add_field(self, register: &[usize], value: &BigUint) -> Self {
        if value.is_zero() { return self; }
        let len = register.len();
        let mask = (BigUint::one() << len) - BigUint::one();
        WorkBag(self.0.into_iter().map(|mut branch| {
            let mut field = BigUint::zero();
            for (index, &wire) in register.iter().enumerate() {
                if branch.value.bit(wire as u64) { field |= BigUint::one() << index as u64; }
            }
            field = (&field + value) & &mask;
            let mut set = BigUint::zero();
            for bit in 0..len {
                if field.bit(bit as u64) { set |= BigUint::one() << bit as u64; }
            }
            for (index, &wire) in register.iter().enumerate() {
                branch.value.set_bit(wire as u64, set.bit(index as u64));
            }
            branch
        }).collect())
    }

    /// Toggle the flag wire exactly where the register field is below value,
    /// reproducing the diagram's XOR'd less-than readout.
    fn compare_field(self, register: &[usize], value: &BigUint, flag: usize) -> Self {
        if value.is_zero() { return self; }
        WorkBag(self.0.into_iter().map(|mut branch| {
            let field = {
                let mut field = BigUint::zero();
                for (index, &wire) in register.iter().enumerate() {
                    if branch.value.bit(wire as u64) {
                        field |= BigUint::one() << index as u64;
                    }
                }
                field
            };
            if field < *value { branch.value ^= BigUint::one() << flag as u64; }
            branch
        }).collect())
    }

    /// `field' = (field + digit * value) mod modulus` per branch. Branches
    /// whose digit bank is zero or whose field sits outside the modulus
    /// interval pass through unchanged, exactly as the diagram cut them.
    fn modular_digit_add_field(self, register: &[usize], digit: &[usize],
        value: &BigUint, modulus: &BigUint) -> Self {
        let reduced = value % modulus;
        if reduced.is_zero() { return self; }
        WorkBag(self.0.into_iter().map(|mut branch| {
            let mut field = BigUint::zero();
            for (index, &wire) in register.iter().enumerate() {
                if branch.value.bit(wire as u64) { field |= BigUint::one() << index as u64; }
            }
            let mut place = BigUint::zero();
            for (index, &wire) in digit.iter().enumerate() {
                if branch.value.bit(wire as u64) {
                    place += BigUint::one() << index as u64;
                }
            }
            if place.is_zero() || field >= *modulus { return branch; }
            let mut addend = &reduced * &place;
            addend %= modulus;
            if addend.is_zero() { return branch; }
            field += addend;
            if field >= *modulus { field -= modulus; }
            for (index, &wire) in register.iter().enumerate() {
                branch.value.set_bit(wire as u64, field.bit(index as u64));
            }
            branch
        }).collect())
    }

    /// Entry i,j of the shared-work Gram trace, summed over branches with
    /// equal values: Re = lr*rr + li*ri, Im = lr*ri - li*rr.
    fn control_gram(roots: &[Self; 4]) -> [(BigInt, BigInt); 16] {
        core::array::from_fn(|index| {
            let left = &roots[index / 4];
            let right = &roots[index % 4];
            let mut re = BigInt::zero();
            let mut im = BigInt::zero();
            let (mut i, mut j) = (0usize, 0usize);
            while i < left.0.len() && j < right.0.len() {
                match left.0[i].value.cmp(&right.0[j].value) {
                    core::cmp::Ordering::Less => i += 1,
                    core::cmp::Ordering::Greater => j += 1,
                    core::cmp::Ordering::Equal => {
                        let lr = left.0[i].re.to_bigint();
                        let li = left.0[i].im.to_bigint();
                        let rr = right.0[j].re.to_bigint();
                        let ri = right.0[j].im.to_bigint();
                        re += &lr * &rr;
                        re += &li * &ri;
                        im += &lr * &ri;
                        im -= &li * &rr;
                        i += 1;
                        j += 1;
                    }
                }
            }
            (re, im)
        })
    }
}

pub struct PreparedModularWork {
    pub source: BigUint,
    pub digit_bits: usize,
    pub operations: Vec<NestedOperation>,
    pub stages: Vec<(BigUint,Vec<usize>)>,
}
impl PreparedModularWork {
    /// Return the already-wired operation references as the two control lanes
    /// at each radix-four phase position. Recovery is checked against the
    /// source-bound emitter before any reference is exposed to execution.
    pub fn validated_lane_stages(
        &self,
        source: &BigUint,
        base: &BigUint,
        digit_bits: usize,
        digits: usize,
    ) -> Result<Vec<(BigUint, Vec<NestedOperation>, Vec<NestedOperation>)>, String> {
        if self.source != *source || self.digit_bits != digit_bits || digit_bits == 0 {
            return Err("prepared work source or radix differs from the resident register".into());
        }
        let source_word = WordTape::from_biguint(source);
        let base_word = WordTape::from_biguint(base);
        let mut gcd = source_word.clone();
        let mut remainder = base_word.clone();
        while !remainder.is_zero() {
            let next = gcd.divmod(&remainder).unwrap().1;
            gcd = remainder; remainder = next;
        }
        if !source_word.ge(&WordTape::from_small(2)) || !base_word.gt(&WordTape::one())
            || base_word.ge(&source_word) || !gcd.is_one()
        {
            return Err("prepared work base must be a source-bound unit".into());
        }
        if self.stages.len() != digits {
            return Err("prepared work has incomplete stack height".into());
        }
        let arithmetic = ModularMultiply::new_phase_register(source)?;
        let mut result = Vec::with_capacity(digits);
        let mut expected = base_word.divmod(&source_word).unwrap().1;
        for (height, (multiplier, references)) in self.stages.iter().enumerate() {
            if WordTape::from_biguint(multiplier) != expected {
                return Err(alloc::format!("prepared work multiplier changed at stack height {}", height));
            }
            let mut lanes: [Vec<NestedOperation>; 2] = [Vec::new(), Vec::new()];
            let square = expected.mul(&expected).divmod(&source_word).unwrap().1;
            for (lane, power) in [(0, expected.clone()), (1, square.clone())] {
                arithmetic.emit_ququart_nested_radix(&power.to_biguint(), lane, digit_bits, |operation| {
                    lanes[lane].push(operation);
                    Ok(())
                })?;
            }
            let expected_len = lanes[0].len() + lanes[1].len();
            if references.len() != expected_len {
                return Err(alloc::format!("prepared work has an incomplete operation stage at stack height {}", height));
            }
            for (position, (id, expected_op)) in references.iter().zip(lanes.iter().flatten()).enumerate() {
                if self.operations.get(*id) != Some(expected_op) {
                    return Err(alloc::format!("prepared work recovery changed operation {} at stack height {}", position, height));
                }
            }
            result.push((multiplier.clone(), lanes[0].clone(), lanes[1].clone()));
            expected = square.mul(&square).divmod(&source_word).unwrap().1;
        }
        Ok(result)
    }

    /// Recover every source-bound operation at every phase-stack stage. This
    /// checks the executable map, not just the shape of the serialized pool.
    pub fn verify_recovery(&self, source: &BigUint, base: &BigUint, digit_bits: usize, digits: usize)
        -> Result<(),String>
    {
        self.validated_lane_stages(source, base, digit_bits, digits).map(|_| ())
    }
}

#[derive(Clone)]
pub struct SicControlWitness {
    pub gram: [(BigInt,BigInt);16],
    pub masses: [BigUint;16],
    pub digit: QuquartDigit,
    pub reconstruction: crate::anyon_ququart::SicGramReconstruction,
}

#[derive(Clone)]
pub struct SicOutcomeWitness {
    pub gram: [(BigInt, BigInt); 16],
    /// Projector masses computed from the measured control Gram.
    pub masses: [BigUint; 16],
    /// Rounded branch norms actually used by the outcome sampler.
    pub sample_masses: [BigUint; 16],
    pub projection_residual: BigUint,
    pub outcome: QuquartSicOutcome,
    pub reconstruction: crate::anyon_ququart::SicGramReconstruction,
}

pub struct FrobeniusStageWitness {
    pub height: usize,
    pub operations: usize,
}
pub struct QuquartFoldedWorkDevice {
    source: BigUint,
    format: FixedPointFormat,
    fourier: PairMatrix,
    sic: FixedQuquartSic,
    cells: usize,
    roots: [WorkBag; 5],
    random: u64,
    active: bool,
    measured: Option<QuquartDigit>,
    expected: usize,
    count: usize,
    interleaved_work: bool,
    work_order: Vec<usize>,
    digit_bits: usize,
    prepared_work: Option<PreparedModularWork>,
    sic_witnesses: Vec<SicControlWitness>,
    sic_outcome_witnesses: Vec<SicOutcomeWitness>,
    nested_since_fold: usize,
    pub peak_nodes: usize,
}
impl QuquartFoldedWorkDevice {
    pub fn new(source: BigUint, fourier: PairMatrix, seed: u64) -> Result<Self, String> {
        let format = FixedPointFormat::for_modulus(&source)?;
        let sic = FixedQuquartSic::new(&format)?;
        let cells = ModularMultiply::new(&source)?.elementary_qubits() - 1;
        Ok(Self {
            format,
            source,
            fourier,
            sic,
            cells,
            roots: core::array::from_fn(|_| WorkBag::zero()),
            random: seed.max(1),
            active: false,
            measured: None,
            expected: 0,
            count: 0,
            interleaved_work: false,
            work_order: vox_core::ordering::selected("work-wires", cells)?.inverse().indices,
            digit_bits: 1,
            prepared_work: None,
            sic_witnesses: Vec::new(),
            sic_outcome_witnesses: Vec::new(),
            nested_since_fold: 0,
            peak_nodes: 0,
        })
    }
    pub fn with_prepared_work(mut self, work: PreparedModularWork) -> Self {
        self.prepared_work = Some(work);
        self
    }
    pub fn sic_witnesses(&self) -> &[SicControlWitness] { &self.sic_witnesses }
    pub fn sic_outcome_witnesses(&self) -> &[SicOutcomeWitness] { &self.sic_outcome_witnesses }

    /// Execute every source-bound stage on a correlated complex state, then
    /// return through the inverse of each adjacent operation. Expected forward
    /// roots are constructed independently from the modular permutation. Root
    /// equality compares the complete carrier bags, including workspace and
    /// the fifth fusion channel. This audit emits no measured phase digits.
    pub fn audit_frobenius_stack(&mut self, base: &BigUint)
        -> Result<Vec<FrobeniusStageWitness>, String>
    {
        if self.active || self.source.bits() < 200 {
            return Err("stack audit requires an inactive RSA source of at least 200 bits".into());
        }
        let schedule = crate::ququart_factor::QuquartPowerSchedule::prepare(&self.source, base)?;
        if let Some(work) = &self.prepared_work {
            work.verify_recovery(&self.source, base, self.digit_bits, schedule.powers().len())?;
        }
        let arithmetic = ModularMultiply::new(&self.source)?;
        let mut witnesses = Vec::new();
        for (height, multiplier) in schedule.powers().iter().enumerate() {
            self.roots = core::array::from_fn(|_| WorkBag::zero());
            self.roots = self.audit_roots(&BigUint::one());
            self.active = true;
            self.measured = None;
            self.nested_since_fold = 0;
            self.fold();
            let operations = if let Some(work) = &self.prepared_work {
                work.stages[height].1.iter().map(|&id| work.operations[id].clone())
                    .collect::<Vec<_>>()
            } else {
                let mut operations = Vec::new();
                for (lane, power) in [(0, multiplier.clone()), (1, multiplier * multiplier % &self.source)] {
                    arithmetic.emit_ququart_nested_radix(&power, lane, self.digit_bits, |operation| {
                        operations.push(operation);
                        Ok(())
                    })?;
                }
                operations
            };
            // Exercise the production dispatch, including the decoded pool.
            self.controlled_multiply(multiplier)?;
            let expected = self.audit_roots(multiplier);
            if self.roots != expected {
                return Err(alloc::format!("transformed work differs at stack height {height}"));
            }
            // The forward comparison is finished before expected roots can be
            // reclaimed. Return uses the actual adjacent maps in reverse order.
            for operation in operations.iter().rev() {
                self.apply_nested(operation.inverse())?;
            }
            self.fold();
            self.require_clean_workspace()?;
            let returned = self.audit_roots(&BigUint::one());
            if self.roots != returned {
                return Err(alloc::format!("transformed work did not return at stack height {height}"));
            }
            witnesses.push(FrobeniusStageWitness { height, operations: operations.len() });
            self.active = false;
        }
        self.abort();
        Ok(witnesses)
    }

    fn audit_roots(&mut self, multiplier: &BigUint) -> [WorkBag; 5] {
        let mut roots = core::array::from_fn(|_| WorkBag::zero());
        let residues = [BigUint::one(), &self.source - 1u8, &self.source - 2u8];
        for channel in 0..5usize {
            let digit = COMPUTATIONAL_CHANNELS.iter().position(|&c| c == channel);
            let power = multiplier.modpow(&BigUint::from(digit.unwrap_or(0)), &self.source);
            for (index, residue) in residues.iter().enumerate() {
                let image = residue * &power % &self.source;
                let scalar = FixedComplex { re: BigInt::from(3 + 11 * channel + index),
                    im: -BigInt::from(7 + 13 * channel + 3 * index) };
                let bag = WorkBag::basis(&self.work_address(&image), &self.format.scale())
                    .scale(&scalar, &self.format);
                roots[channel] = core::mem::replace(&mut roots[channel], WorkBag::zero())
                    .merge(bag);
            }
        }
        roots
    }
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
        if wire < 2 { return wire; }
        let width = self.source.bits() as usize;
        let mapped = if !self.interleaved_work || wire > 2*width+1 {wire}
            else if wire <= width+1 {2*(wire-1)} else {2*(wire-width-1)+1};
        // The selected sequence lists the logical coordinate at each physical position.
        2 + self.work_order[mapped-2]
    }
    fn work_address(&self, residue: &BigUint) -> BigUint {
        let mut address = BigUint::zero();
        for bit in 0..residue.bits() {
            address.set_bit((self.work_wire(bit as usize+2)-2) as u64, residue.bit(bit));
        }
        address
    }
    fn require_clean_workspace(&self) -> Result<(),String> {
        let width = self.source.bits() as usize;
        for (channel, root) in self.roots.iter().enumerate() {
            // Check the whole allocated stack: work accumulator, borrow flag,
            // ripple carries, and control-lowering ancilla. The fifth fusion
            // channel carries workspace too, even outside the computation.
            for logical in width+2..self.cells+2 {
                let wire = self.work_wire(logical)-2;
                if !root.wire_is_zero(wire) {
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
        for root in &mut self.roots {
            *root = core::mem::take(root).deduped();
        }
        self.nested_since_fold = 0;
        let retained: usize = self.roots.iter().map(|root| root.0.len()).sum();
        self.peak_nodes = self.peak_nodes.max(retained);
        VOX_QUQUART_COUNTERS[4].store(retained as u64, Ordering::Relaxed);
        VOX_QUQUART_COUNTERS[5].store(self.peak_nodes as u64, Ordering::Relaxed);
    }
    /// Live branch count per root, for execution telemetry only.
    pub fn bag_profile(&self) -> [usize; 5] {
        core::array::from_fn(|index| self.roots[index].0.len())
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
        if target >= self.cells + 2
            || controls
                .iter()
                .any(|&q| q >= self.cells + 2 || q == target)
        {
            return Err("reversible gate has invalid ququart work wires".into());
        }
        let mut work: Vec<_> = controls
            .iter()
            .filter(|&&q| q >= 2)
            .map(|&q| q - 2)
            .collect();
        work.sort_unstable();
        let work: Vec<(usize, bool)> = work.iter().map(|&wire| (wire, true)).collect();
        let old: [WorkBag; 5] = core::array::from_fn(|index| core::mem::take(&mut self.roots[index]));
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            self.roots[channel] = old[channel].clone();
            if controls.iter().any(|&q| q < 2 && digit & (1 << q) == 0) {
                continue;
            }
            self.roots[channel] = if target < 2 {
                old[COMPUTATIONAL_CHANNELS[digit ^ (1 << target)]].clone()
            } else {
                let (enabled, rest) = old[channel].split_literals(&work);
                rest.merge(enabled.toggle_bit(target - 2))
            };
        }
        Ok(())
    }

    fn apply_nested(&mut self, operation: NestedOperation) -> Result<(), String> {
        VOX_QUQUART_COUNTERS[3].fetch_add(1, Ordering::Relaxed);
        let variant = match &operation {
            NestedOperation::Toggle(_) => 0,
            NestedOperation::ModularAdd { .. } => 1,
            NestedOperation::Add { .. } => 2,
            NestedOperation::Compare { .. } => 3,
        };
        VOX_NESTED_OPERATION_COUNTERS[variant].fetch_add(1, Ordering::Relaxed);
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
        let extent = self.cells + 2;
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
        let old: [WorkBag; 5] = core::array::from_fn(|index| core::mem::take(&mut self.roots[index]));
        for (digit, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let incoming = match &operation {
                NestedOperation::Toggle(gate) if gate.target < 2 =>
                    old[COMPUTATIONAL_CHANNELS[digit ^ (1 << gate.target)]].clone(),
                _ => old[channel].clone(),
            };
            if controls.iter().any(|&(wire,value)| wire < 2 && (digit & (1 << wire) != 0) != value) {
                self.roots[channel] = old[channel].clone();
                continue;
            }
            if !old[channel].controls_satisfiable(&work) && !incoming.controls_satisfiable(&work) {
                self.roots[channel] = old[channel].clone();
                continue;
            }
            self.roots[channel] = match &operation {
                NestedOperation::Toggle(gate) => {
                    if gate.target < 2 {
                        incoming
                    } else {
                        let (enabled, rest) = old[channel].split_literals(&work);
                        rest.merge(enabled.toggle_bit(gate.target - 2))
                    }
                }
                NestedOperation::ModularAdd { register,digit,value,modulus,.. } => {
                    let (enabled, rest) = old[channel].split_literals(&work);
                    rest.merge(enabled.modular_digit_add_field(
                        &register.iter().map(|wire| wire-2).collect::<Vec<_>>(),
                        &digit.iter().map(|wire| wire-2).collect::<Vec<_>>(),
                        value, modulus))
                }
                NestedOperation::Add { register,value,.. } => {
                    let (enabled, rest) = old[channel].split_literals(&work);
                    rest.merge(enabled.add_field(
                        &register.iter().map(|wire| wire-2).collect::<Vec<_>>(), value))
                }
                NestedOperation::Compare { register,value,flag,.. } => {
                    let (enabled, rest) = old[channel].split_literals(&work);
                    rest.merge(enabled.compare_field(
                        &register.iter().map(|wire| wire-2).collect::<Vec<_>>(), value, flag-2))
                }
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
        let gram = WorkBag::control_gram(&core::array::from_fn(|digit|
            self.roots[COMPUTATIONAL_CHANNELS[digit]].clone()));
        let gram_masses = self.sic.gram_masses(&gram)?;
        let reconstruction = self.sic.certify_gram_frame(&gram, &gram_masses)?;
        let old: [WorkBag; 5] = core::array::from_fn(|index| self.roots[index].clone());
        let mut branches: [Option<WorkBag>; 16] = core::array::from_fn(|_| None);
        let mut masses: [BigUint; 17] = core::array::from_fn(|_| BigUint::zero());
        for (outcome, ray) in self.sic.rays().iter().enumerate() {
            let mut branch = WorkBag::zero();
            for (component, &channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let scalar = crate::phase_unbraid::FixedComplex {
                    re: ray[component].re.clone(),
                    im: -&ray[component].im,
                };
                branch = branch.merge(old[channel].clone().scale(&scalar, &self.format));
            }
            masses[outcome] = branch.mass();
            branches[outcome] = Some(branch);
        }
        masses[16] = self.roots[LEAKAGE_CHANNEL].mass() * 4u8;
        let sample_masses: [BigUint; 16] = core::array::from_fn(|index| masses[index].clone());
        let projection_residual = gram_masses.iter().zip(&sample_masses).map(|(analytic, sampled)|
            if analytic >= sampled { analytic - sampled } else { sampled - analytic })
            .max().unwrap_or_else(BigUint::zero);
        if BigInt::from(projection_residual.clone()) > reconstruction.tolerance {
            return Err("SIC projection rounding exceeds its source-bound Gram tolerance".into());
        }
        let outcome = crate::anyon_fusion_kernel::sample_born_masses(&masses, |bytes| {
            Self::entropy(&mut self.random, bytes)
        })?;
        let measured = if outcome == 16 { QuquartSicOutcome::OutsideCarrier } else {
            QuquartSicOutcome::Carrier(crate::sic::SixteenOutcome::new(outcome as u8)
                .map_err(|error| error.to_string())?)
        };
        self.sic_outcome_witnesses.push(SicOutcomeWitness {
            gram, masses: gram_masses, sample_masses, projection_residual,
            outcome: measured, reconstruction,
        });
        if outcome == 16 {
            self.roots = core::array::from_fn(|_| WorkBag::zero());
            self.fold();
            self.count += 1;
            return Ok(QuquartSicOutcome::OutsideCarrier);
        }
        let norm = BigInt::from(masses[outcome].sqrt());
        if norm.is_zero() {
            return Err("selected SIC outcome has zero Born mass".into());
        }
        let normalized = branches[outcome].take().unwrap()
            .normalize(&norm, &self.format.scale());
        self.roots = core::array::from_fn(|_| WorkBag::zero());
        self.roots[COMPUTATIONAL_CHANNELS[0]] = normalized;
        self.fold();
        self.count += 1;
        Ok(measured)
    }
}

impl WorkBag {
    /// Per-branch normalized readout: each numerator is rescaled by
    /// scale/norm with the same signed-word division the diagram used.
    fn normalize(self, norm: &BigInt, scale: &BigInt) -> Self {
        let norm = SignedWordTape::from_bigint(norm);
        let scale = SignedWordTape::from_bigint(scale);
        WorkBag(self.0.into_iter().map(|branch| WorkBranch {
            re: branch.re.mul(&scale).div(&norm),
            im: branch.im.mul(&scale).div(&norm),
            value: branch.value,
        }).collect()).deduped()
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
        self.roots = core::array::from_fn(|_| WorkBag::zero());
        self.roots[COMPUTATIONAL_CHANNELS[0]] = WorkBag::basis(
            &self.work_address(register.residue()), &self.format.scale());
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
        let old: [WorkBag; 5] = core::array::from_fn(|index| core::mem::take(&mut self.roots[index]));
        for row in 0..5 {
            let mut sum = WorkBag::zero();
            for (col, root) in old.iter().enumerate() {
                let term = root.clone().scale(&matrix.0[5 * row + col], &self.format);
                sum = sum.merge(term);
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
            self.roots[channel] = core::mem::take(&mut self.roots[channel]).scale(&phase, &self.format);
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
            self.roots[if digit < 4 {
                COMPUTATIONAL_CHANNELS[digit]
            } else {
                LEAKAGE_CHANNEL
            }].mass()
        });
        let gram = WorkBag::control_gram(&core::array::from_fn(|digit|
            self.roots[COMPUTATIONAL_CHANNELS[digit]].clone()));
        let sic_masses = self.sic.gram_masses(&gram)?;
        let reconstruction = self.sic.certify_gram_frame(&gram,&sic_masses)?;
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
        let root = self.roots[selected].clone().normalize(&norm, &self.format.scale());
        self.roots = core::array::from_fn(|_| WorkBag::zero());
        self.roots[selected] = root;
        let digit = QuquartDigit::try_from(outcome as u8)?;
        self.sic_witnesses.push(SicControlWitness { gram, masses:sic_masses, digit, reconstruction });
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
        let root = core::mem::take(&mut self.roots[COMPUTATIONAL_CHANNELS[digit as usize]]);
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
        self.roots = core::array::from_fn(|_| WorkBag::zero());
        self.active = false;
        self.measured = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
    use num_bigint::BigInt;

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
                let clean = WorkBag::basis(&device.work_address(&(&n-1u8)), &format.scale());
                for channel in 0..5 {
                    device.roots[channel] = clean.clone();
                }
                assert!(device.require_clean_workspace().is_ok());
                // Exercise every workspace height, including the previously
                // unchecked borrow flag and the top control-lowering ancilla.
                for logical in n.bits() as usize+2..device.cells+2 {
                    let dirty = clean.clone().toggle_bit(device.work_wire(logical)-2);
                    for channel in 0..5 {
                        device.roots[channel] = dirty.clone();
                        assert!(device.require_clean_workspace().is_err(),
                            "upper workspace in channel {channel} was not checked");
                        device.roots[channel] = clean.clone();
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
        device.roots = core::array::from_fn(|_| WorkBag::zero());
        let residues = [&n-1u8,&n-2u8];
        for (digit,&channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (index,residue) in residues.iter().enumerate() {
                let scalar = FixedComplex { re: BigInt::from(3+5*digit+index), im: BigInt::from(7+3*digit+2*index) };
                let branch = WorkBag::basis(&device.work_address(residue), &format.scale())
                    .scale(&scalar, &format);
                device.roots[channel] = device.roots[channel].clone().merge(branch);
            }
        }
        device.fold();
        let masses: Vec<_> = COMPUTATIONAL_CHANNELS.iter().map(|&c| device.roots[c].mass()).collect();
        let schedule = crate::ququart_factor::QuquartPowerSchedule::prepare(&n,&BigUint::from(2u8)).unwrap();
        let multiplier = &schedule.powers()[64];
        device.controlled_multiply(multiplier).unwrap();
        let mut power = BigUint::one();
        for (digit,&channel) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (index,residue) in residues.iter().enumerate() {
                let image = residue*&power%&n;
                assert_eq!(device.roots[channel].amplitude(&device.work_address(&image)),
                    (BigInt::from(3+5*digit+index),BigInt::from(7+3*digit+2*index)));
            }
            // Exact mass at the clean-work addresses exhausts the channel:
            // no amplitude can remain in any arithmetic or carry ancilla.
            assert_eq!(device.roots[channel].mass(),masses[digit]);
            power = power*multiplier%&n;
        }
        assert!(device.roots[LEAKAGE_CHANNEL].mass().is_zero());
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
        let witness = device.sic_outcome_witnesses().last().unwrap();
        assert_eq!(witness.outcome, outcome);
        assert_eq!(device.sic.gram_masses(&witness.gram).unwrap(), witness.masses);
        assert_eq!(device.sic.certify_gram_frame(&witness.gram, &witness.masses).unwrap(),
            witness.reconstruction);
        let measured_residual = witness.masses.iter().zip(&witness.sample_masses)
            .map(|(analytic, sampled)| if analytic >= sampled { analytic - sampled } else { sampled - analytic })
            .max().unwrap();
        assert_eq!(measured_residual, witness.projection_residual);
        assert!(BigInt::from(measured_residual) <= witness.reconstruction.tolerance);
        assert_eq!(device.roots[COMPUTATIONAL_CHANNELS[0]].mass(), format.scale().to_biguint().unwrap().pow(2));
        for &channel in &COMPUTATIONAL_CHANNELS[1..] {
            assert!(device.roots[channel].mass().is_zero());
        }
        assert!(device.roots[LEAKAGE_CHANNEL].mass().is_zero());
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
            let (re, im) = device.roots[channel].amplitude(&image.clone());
            assert_eq!(re, format.scale() / 2u8);
            assert!(im.is_zero());
            assert_eq!(
                device.roots[channel].mass(),
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
        let (re, im) = device.roots[COMPUTATIONAL_CHANNELS[0]].amplitude(x);
        assert_eq!(re, format.scale());
        assert!(im.is_zero());
        for &channel in &COMPUTATIONAL_CHANNELS[1..] {
            assert!(device.roots[channel].mass().is_zero());
        }
    }


}
