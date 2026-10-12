//! One-shot phase estimation over a resident powerset decision register.
//!
//! The phase and work wires share one complex decision diagram. Controlled
//! modular powers and the complete inverse Fourier transform act on that
//! diagram before one full-width Born readout. The correlated powerset itself
//! is the re-entry state; no branch is detached for digitwise measurement or
//! feedback during execution.

use crate::phase_unbraid::FixedPointFormat;
use crate::phase_unbraid::PhaseReadoutAccumulator;
use crate::ququart_decision::DecisionArena;
use crate::ququart_folded_work::PreparedModularWork;
use crate::reversible_modular::{ElementaryGate, ModularMultiply, NestedOperation};
use alloc::string::String;
use num_bigint::BigUint;
use num_traits::{One, Zero};

fn fold_diagram(arena: &mut DecisionArena, root: &mut usize) {
    let zero = arena.zero();
    let mut roots = [zero; 5];
    roots[0] = *root;
    arena.fold(&mut roots);
    *root = roots[0];
}

fn apply_elementary(
    arena: &mut DecisionArena,
    root: &mut usize,
    width: usize,
    control: usize,
    gate: ElementaryGate,
) -> Result<(), String> {
    let map_wire = |wire: usize| -> Result<usize, String> {
        if wire == 0 {
            Ok(control)
        } else {
            width
                .checked_add(wire - 1)
                .ok_or_else(|| "modular gate wire overflow".into())
        }
    };
    let (target, mut controls) = match gate {
        ElementaryGate::X(target) => (map_wire(target)?, Vec::new()),
        ElementaryGate::Cnot { control, target } => {
            (map_wire(target)?, alloc::vec![map_wire(control)?])
        }
        ElementaryGate::Toffoli {
            first,
            second,
            target,
        } => (
            map_wire(target)?,
            alloc::vec![map_wire(first)?, map_wire(second)?],
        ),
    };
    controls.sort_unstable();
    if controls.iter().any(|&wire| wire == target)
        || controls.windows(2).any(|pair| pair[0] == pair[1])
        || target >= arena.cells
        || controls.iter().any(|&wire| wire >= arena.cells)
    {
        return Err("controlled modular gate escaped the prepared register".into());
    }
    *root = arena.controlled_flip(*root, target, &controls);
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeRegisterPhaseReadout {
    pub numerator: BigUint,
    pub denominator: BigUint,
    pub width: usize,
    pub source: BigUint,
    pub base: BigUint,
}

pub struct WholeRegisterFactorShot {
    pub phase: WholeRegisterPhaseReadout,
    pub closure: Option<crate::ququart_factor::QuquartFactorClosure>,
}

/// Source-bound factor route whose phase evidence enters as one complete
/// register sample and closes through the existing winding membrane.
pub struct WholeRegisterPhaseExecutor {
    source: BigUint,
    base: BigUint,
    width: Option<usize>,
    orientation: Vec<(bool, bool)>,
    random: u64,
    evidence: PhaseReadoutAccumulator,
    prepared_work: Option<PreparedModularWork>,
    streaming_digit_bits: Option<usize>,
    pending_circuit: Option<WholeRegisterPhaseCircuit>,
}

impl WholeRegisterPhaseExecutor {
    pub fn prepare(source: BigUint, base: BigUint, width: Option<usize>) -> Result<Self, String> {
        Self::prepare_with_reentry(source, base, width, 1729, &[])
    }

    /// Bind the Y/re-entry orientation word to the phase control coordinates.
    /// Each pair labels the two bits of one radix-four phase position.
    pub fn prepare_with_reentry(
        source: BigUint,
        base: BigUint,
        width: Option<usize>,
        seed: u64,
        orientation: &[(bool, bool)],
    ) -> Result<Self, String> {
        Self::prepare_with_reentry_and_work(source, base, width, seed, orientation, None)
    }

    /// Bind the prepared rewirer pool to the complete phase register. Every
    /// operation reference is recovered against the source-bound power stack
    /// before the executor can run.
    pub fn prepare_with_reentry_and_work(
        source: BigUint,
        base: BigUint,
        width: Option<usize>,
        seed: u64,
        orientation: &[(bool, bool)],
        prepared_work: Option<PreparedModularWork>,
    ) -> Result<Self, String> {
        let resolved_width = match width {
            Some(width) => width,
            None => usize::try_from(
                source
                    .bits()
                    .checked_mul(2)
                    .ok_or("phase register width overflow")?,
            )
            .map_err(|_| "phase register width exceeds host indexing")?,
        };
        if !orientation.is_empty() && orientation.len().checked_mul(2) != Some(resolved_width) {
            return Err("Y/re-entry orientation must cover the complete phase register".into());
        }
        // Validate the entire circuit before admitting this source/base pair.
        WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            source.clone(),
            base.clone(),
            resolved_width,
            orientation,
            prepared_work.as_ref(),
        )?;
        Ok(Self {
            source,
            base,
            width: Some(resolved_width),
            orientation: orientation.to_vec(),
            random: seed.max(1),
            evidence: PhaseReadoutAccumulator::default(),
            prepared_work,
            streaming_digit_bits: None,
            pending_circuit: None,
        })
    }

    /// Use the existing nested modular emitter directly, without storing an
    /// externally prepared operation pool or changing the joint readout.
    pub fn use_streaming_nested_work(&mut self, digit_bits: usize) -> Result<(), String> {
        if digit_bits == 0 || self.prepared_work.is_some() {
            return Err("streaming nested work requires a positive radix and no external pool".into());
        }
        self.streaming_digit_bits = Some(digit_bits);
        Ok(())
    }

    /// Retain the complete powerset between operation returns. This performs
    /// no intermediate phase measurements; readout occurs after the full QFT.
    pub fn advance_streaming_next(&mut self) -> Result<Option<WholeRegisterFactorShot>, String> {
        let bits = self.streaming_digit_bits.ok_or("streaming work has not been selected")?;
        if self.pending_circuit.is_none() {
            self.pending_circuit = Some(WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
                self.source.clone(), self.base.clone(), self.width.unwrap(),
                &self.orientation, None)?);
        }
        if !self.pending_circuit.as_mut().unwrap().advance_nested_work(bits, 8)? {
            return Ok(None);
        }
        let mut circuit = self.pending_circuit.take().unwrap();
        let denominator = BigUint::one() << self.width.unwrap();
        let mut quantile = BigUint::zero();
        for offset in (0..self.width.unwrap()).step_by(64) {
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            quantile |= BigUint::from(self.random) << offset;
        }
        quantile %= &denominator;
        let phase = circuit.fire_with_work_radix(&quantile, &denominator, Some(bits))?;
        self.close_readout(phase).map(Some)
    }

    /// Execution coordinates only; observing these does not measure a phase.
    pub fn streaming_progress(&self) -> Option<(usize, usize, usize)> {
        let circuit = self.pending_circuit.as_ref()?;
        let work = circuit.streaming_work.as_ref()?;
        Some((work.phase_wire, work.cursor, circuit.peak_nodes()))
    }

    /// Draw one complete-register sample from the executor's retained
    /// measurement stream. The seed initializes the stream once, not each shot.
    pub fn measure_next(&mut self) -> Result<WholeRegisterFactorShot, String> {
        self.measure_next_with_readout(|_| Ok(()))
    }

    /// Expose the complete measured register through a terminal port before
    /// feeding its value into the factor-closure operation.
    pub fn measure_next_with_readout(
        &mut self,
        readout: impl FnOnce(&WholeRegisterPhaseReadout) -> Result<(), String>,
    ) -> Result<WholeRegisterFactorShot, String> {
        let width = self.width.expect("prepared phase width");
        let denominator = BigUint::one() << width;
        let mut numerator = BigUint::zero();
        let mut offset = 0usize;
        while offset < width {
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            numerator |= BigUint::from(self.random) << offset;
            offset = offset.saturating_add(64);
        }
        numerator %= &denominator;
        self.measure_with_readout(&numerator, &denominator, readout)
    }

    pub fn measure(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
    ) -> Result<WholeRegisterFactorShot, String> {
        self.measure_with_readout(quantile, quantile_denominator, |_| Ok(()))
    }

    pub fn measure_with_readout(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
        readout: impl FnOnce(&WholeRegisterPhaseReadout) -> Result<(), String>,
    ) -> Result<WholeRegisterFactorShot, String> {
        let mut circuit = WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            self.source.clone(),
            self.base.clone(),
            self.width.unwrap(),
            &self.orientation,
            self.prepared_work.as_ref(),
        )?;
        let phase = circuit.fire_with_work_radix(quantile, quantile_denominator,
            self.streaming_digit_bits)?;
        readout(&phase)?;
        self.close_readout(phase)
    }

    fn close_readout(&mut self, phase: WholeRegisterPhaseReadout)
        -> Result<WholeRegisterFactorShot, String> {
        let closure = if let Some((order, measured_p, measured_q)) = self.evidence.close_fraction(
            &phase.numerator,
            &phase.denominator,
            &self.base,
            &self.source,
        )? {
            let result = crate::ququart_factor::extract_certified_winding_for_base(
                &self.source,
                &self.base,
                &order,
            )?;
            let (p, q) = result.factors();
            if !((p == &measured_p && q == &measured_q) || (p == &measured_q && q == &measured_p)) {
                return Err("whole-register winding changed its measured factor arms".into());
            }
            Some(result)
        } else {
            None
        };
        Ok(WholeRegisterFactorShot { phase, closure })
    }
}

/// Prepared source-bound phase circuit. The powerset IS re-entry: one
/// correlated decision object holds every phase branch as the controlled
/// modular operator acts with successive squared bases.
pub struct WholeRegisterPhaseCircuit {
    source: BigUint,
    base: BigUint,
    width: usize,
    format: FixedPointFormat,
    arithmetic: ModularMultiply,
    arena: DecisionArena,
    root: usize,
    gates_since_fold: usize,
    peak_nodes: usize,
    fired: bool,
    prepared_stages: Option<Vec<(BigUint, Vec<NestedOperation>, Vec<NestedOperation>)>>,
    streaming_work: Option<StreamingWork>,
}

struct StreamingWork {
    digit_bits: usize,
    phase_wire: usize,
    multiplier: BigUint,
    operations: Vec<NestedOperation>,
    cursor: usize,
}

impl WholeRegisterPhaseCircuit {
    /// Prepare the complete control and work register from the resident
    /// `(base, source)` relation. No phase outcome or period enters preparation.
    pub fn prepare(source: BigUint, base: BigUint, width: Option<usize>) -> Result<Self, String> {
        let resolved_width = match width {
            Some(width) => width,
            None => usize::try_from(
                source
                    .bits()
                    .checked_mul(2)
                    .ok_or("phase register width overflow")?,
            )
            .map_err(|_| "phase register width exceeds host indexing")?,
        };
        Self::prepare_with_reentry(source, base, resolved_width, &[])
    }

    pub fn prepare_with_reentry(
        source: BigUint,
        base: BigUint,
        width: usize,
        orientation: &[(bool, bool)],
    ) -> Result<Self, String> {
        Self::prepare_with_reentry_and_work(source, base, width, orientation, None)
    }

    pub fn prepare_with_reentry_and_work(
        source: BigUint,
        base: BigUint,
        width: usize,
        orientation: &[(bool, bool)],
        prepared_work: Option<&PreparedModularWork>,
    ) -> Result<Self, String> {
        if source < BigUint::from(4u8) {
            return Err("phase register requires a modulus of at least four".into());
        }
        if base <= BigUint::one()
            || base >= source
            || crate::factor_routes::big_gcd(base.clone(), source.clone()) != BigUint::one()
        {
            return Err("phase register base must be a nontrivial source-bound unit".into());
        }
        if width == 0 {
            return Err("phase register width must be positive".into());
        }
        if !orientation.is_empty() && orientation.len().checked_mul(2) != Some(width) {
            return Err("Y/re-entry orientation must cover the complete phase register".into());
        }
        let prepared_stages = prepared_work
            .map(|work| work.validated_lane_stages(&source, &base, work.digit_bits, width / 2))
            .transpose()?;
        if prepared_stages.is_some() && width % 2 != 0 {
            return Err("prepared rewirer requires complete radix-four phase positions".into());
        }
        let arithmetic = ModularMultiply::new_phase_register(&source)?;
        let cells = width
            .checked_add(arithmetic.elementary_qubits().saturating_sub(1))
            .ok_or("whole-register wire layout overflow")?;
        let format = FixedPointFormat::for_modulus(&source)?;
        let mut arena = DecisionArena::new(cells);
        let address = BigUint::one() << width;
        let root = arena.basis(&address, format.scale());
        let mut circuit = Self {
            source,
            base,
            width,
            format,
            arithmetic,
            arena,
            root,
            gates_since_fold: 0,
            peak_nodes: 0,
            fired: false,
            prepared_stages,
            streaming_work: None,
        };
        for wire in 0..width {
            circuit.root = circuit
                .arena
                .hadamard(circuit.root, wire, &circuit.format)?;
            circuit.fold();
        }
        for (position, &(forward, inverse)) in orientation.iter().enumerate() {
            for (lane, active) in [forward, inverse].into_iter().enumerate() {
                if active {
                    let wire = 2 * position + lane;
                    circuit.root = circuit.arena.controlled_flip(circuit.root, wire, &[]);
                }
            }
        }
        circuit.fold();
        Ok(circuit)
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn source(&self) -> &BigUint {
        &self.source
    }
    pub fn base(&self) -> &BigUint {
        &self.base
    }
    pub fn peak_nodes(&self) -> usize {
        self.peak_nodes
    }

    /// Advance operations while the complete phase/work register stays resident.
    /// Only one controlled multiplier's operation pool is retained at a time.
    pub fn advance_nested_work(&mut self, digit_bits: usize, quantum: usize)
        -> Result<bool, String> {
        if self.fired || self.prepared_stages.is_some() || digit_bits == 0 || quantum == 0 {
            return Err("invalid incremental nested-work state".into());
        }
        if self.streaming_work.is_none() {
            self.streaming_work = Some(StreamingWork { digit_bits, phase_wire: 0,
                multiplier: self.base.clone(), operations: Vec::new(), cursor: 0 });
        }
        for _ in 0..quantum {
            let mut work = self.streaming_work.take().unwrap();
            if work.digit_bits != digit_bits {
                self.streaming_work = Some(work);
                return Err("nested-work radix changed during execution".into());
            }
            if work.phase_wire == self.width {
                self.streaming_work = Some(work);
                return Ok(true);
            }
            if work.operations.is_empty() {
                self.arithmetic.emit_ququart_nested_radix(&work.multiplier,
                    work.phase_wire % 2, digit_bits, |op| { work.operations.push(op); Ok(()) })?;
            }
            if !work.operations.is_empty() {
                self.apply_prepared_operation(work.phase_wire, work.phase_wire % 2,
                    &work.operations[work.cursor])?;
                work.cursor += 1;
            }
            if work.cursor == work.operations.len() {
                work.operations.clear();
                work.cursor = 0;
                work.phase_wire += 1;
                work.multiplier = &work.multiplier * &work.multiplier % &self.source;
            }
            self.streaming_work = Some(work);
        }
        Ok(self.streaming_work.as_ref().unwrap().phase_wire == self.width)
    }

    fn fold(&mut self) {
        fold_diagram(&mut self.arena, &mut self.root);
        self.gates_since_fold = 0;
        self.peak_nodes = self.peak_nodes.max(self.arena.retained_nodes());
    }

    fn controlled_multiply(
        &mut self,
        phase_wire: usize,
        multiplier: &BigUint,
    ) -> Result<(), String> {
        let arithmetic = &self.arithmetic;
        let width = self.width;
        let arena = &mut self.arena;
        let root = &mut self.root;
        let gates_since_fold = &mut self.gates_since_fold;
        let peak_nodes = &mut self.peak_nodes;
        arithmetic.emit_elementary(multiplier, |gate| {
            apply_elementary(arena, root, width, phase_wire, gate)?;
            *gates_since_fold += 1;
            if *gates_since_fold == 8 {
                fold_diagram(arena, root);
                *gates_since_fold = 0;
                *peak_nodes = (*peak_nodes).max(arena.retained_nodes());
            }
            Ok(())
        })
    }

    fn apply_prepared_operation(
        &mut self,
        phase_wire: usize,
        lane: usize,
        operation: &NestedOperation,
    ) -> Result<(), String> {
        let map_wire = |wire: usize| -> Result<usize, String> {
            match wire {
                0 | 1 if wire == lane => Ok(phase_wire),
                0 | 1 => Err("prepared operation addresses the other phase lane".into()),
                _ => self
                    .width
                    .checked_add(wire - 2)
                    .ok_or_else(|| "prepared work wire overflow".into()),
            }
        };
        let controls = |items: &[(usize, bool)]| -> Result<Vec<(usize, bool)>, String> {
            items
                .iter()
                .map(|&(wire, positive)| Ok((map_wire(wire)?, positive)))
                .collect()
        };
        self.root = match operation {
            NestedOperation::Toggle(gate) => {
                let target = map_wire(gate.target)?;
                let controls = controls(&gate.controls)?;
                if target >= self.arena.cells
                    || controls.iter().any(|&(wire, _)| wire >= self.arena.cells)
                    || controls.iter().any(|&(wire, _)| wire == target)
                {
                    return Err("prepared toggle escaped its rewirer register".into());
                }
                let zero = self.arena.zero();
                let enabled = self.arena.conditional_literals(zero, self.root, &controls);
                let changed = self.arena.flip(enabled, target);
                self.arena
                    .conditional_literals(self.root, changed, &controls)
            }
            NestedOperation::ModularAdd {
                register,
                digit,
                value,
                modulus,
                controls: literals,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let digit = digit
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let controls = controls(literals)?;
                if register
                    .iter()
                    .chain(digit.iter())
                    .chain(controls.iter().map(|x| &x.0))
                    .any(|&wire| wire >= self.arena.cells)
                {
                    return Err("prepared modular addition escaped its rewirer register".into());
                }
                self.arena
                    .modular_digit_add(self.root, &register, &digit, value, modulus, &controls)
            }
            NestedOperation::Add {
                register,
                value,
                controls: literals,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let controls = controls(literals)?;
                self.arena
                    .add_constant(self.root, &register, value, &controls)
            }
            NestedOperation::Compare {
                register,
                value,
                controls: literals,
                flag,
            } => {
                let register = register
                    .iter()
                    .map(|&wire| map_wire(wire))
                    .collect::<Result<Vec<_>, _>>()?;
                let flag = map_wire(*flag)?;
                let controls = controls(literals)?;
                self.arena
                    .compare_constant(self.root, &register, value, &controls, flag)
            }
        };
        self.gates_since_fold += 1;
        if self.gates_since_fold == 8 {
            self.fold();
        }
        Ok(())
    }

    /// Fire the prepared circuit once and return the entire phase register in
    /// one sample. `quantile` is the Born-rank supplied by the measurement
    /// instrument; the phase word itself is not reconstructed from digits.
    pub fn fire(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
    ) -> Result<WholeRegisterPhaseReadout, String> {
        self.fire_with_work_radix(quantile, quantile_denominator, None)
    }

    pub fn fire_with_work_radix(
        &mut self,
        quantile: &BigUint,
        quantile_denominator: &BigUint,
        streaming_digit_bits: Option<usize>,
    ) -> Result<WholeRegisterPhaseReadout, String> {
        if self.fired {
            return Err("whole-register phase circuit has already fired".into());
        }
        if quantile_denominator.is_zero() || quantile >= quantile_denominator {
            return Err("whole-register phase measurement has an invalid Born quantile".into());
        }
        if let Some(work) = &self.streaming_work {
            let bits = work.digit_bits;
            if streaming_digit_bits != Some(bits) {
                return Err("readout work radix differs from retained work".into());
            }
            while !self.advance_nested_work(bits, 8)? {}
        } else if let Some(stages) = self.prepared_stages.clone() {
            for (position, (_, forward, inverse)) in stages.iter().enumerate() {
                let phase_wires = [2 * position, 2 * position + 1];
                for (lane, (phase_wire, operations)) in
                    phase_wires.into_iter().zip([forward, inverse]).enumerate()
                {
                    for operation in operations {
                        self.apply_prepared_operation(phase_wire, lane, operation)?;
                    }
                }
            }
        } else if let Some(digit_bits) = streaming_digit_bits {
            if digit_bits == 0 {
                return Err("streaming nested work radix must be positive".into());
            }
            let arithmetic = ModularMultiply::new_phase_register(&self.source)?;
            let mut multiplier = self.base.clone();
            for phase_wire in 0..self.width {
                let lane = phase_wire % 2;
                arithmetic.emit_ququart_nested_radix(&multiplier, lane, digit_bits,
                    |operation| self.apply_prepared_operation(phase_wire, lane, &operation))?;
                multiplier = &multiplier * &multiplier % &self.source;
            }
        } else {
            let mut multiplier = self.base.clone();
            for phase_wire in 0..self.width {
                self.controlled_multiply(phase_wire, &multiplier)?;
                multiplier = &multiplier * &multiplier % &self.source;
            }
        }
        self.fold();
        let phase_wires: Vec<_> = (0..self.width).collect();
        self.root = self
            .arena
            .fourier_register(self.root, &phase_wires, true, &self.format)?;
        self.fold();
        let numerator =
            self.arena
                .sample_prefix(self.root, self.width, quantile, quantile_denominator)?;
        self.fired = true;
        Ok(WholeRegisterPhaseReadout {
            numerator,
            denominator: BigUint::one() << self.width,
            width: self.width,
            source: self.source.clone(),
            base: self.base.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_work(source: &BigUint, base: &BigUint, digit_bits: usize) -> PreparedModularWork {
        let arithmetic = ModularMultiply::new_phase_register(source).unwrap();
        let mut operations = Vec::new();
        let mut stages = Vec::new();
        let mut multiplier = base % source;
        for _ in 0..2 {
            let start = operations.len();
            for (lane, power) in [
                (0, multiplier.clone()),
                (1, &multiplier * &multiplier % source),
            ] {
                arithmetic
                    .emit_ququart_nested_radix(&power, lane, digit_bits, |operation| {
                        operations.push(operation);
                        Ok(())
                    })
                    .unwrap();
            }
            stages.push((multiplier.clone(), (start..operations.len()).collect()));
            let square = &multiplier * &multiplier % source;
            multiplier = &square * &square % source;
        }
        PreparedModularWork {
            source: source.clone(),
            digit_bits,
            operations,
            stages,
        }
    }

    #[test]
    fn complete_register_readout_matches_all_four_dense_period_bins() {
        for (quantile, expected) in [(1u8, 0u8), (3, 8), (5, 4), (7, 12)] {
            let mut circuit = WholeRegisterPhaseCircuit::prepare(
                BigUint::from(15u8),
                BigUint::from(2u8),
                Some(4),
            )
            .unwrap();
            let readout = circuit
                .fire(&BigUint::from(quantile), &BigUint::from(8u8))
                .unwrap();
            assert_eq!(readout.width, 4);
            assert_eq!(readout.denominator, BigUint::from(16u8));
            assert_eq!(readout.numerator, BigUint::from(expected));
            assert_eq!(readout.source, BigUint::from(15u8));
            assert_eq!(readout.base, BigUint::from(2u8));
            assert!(circuit.fire(&BigUint::zero(), &BigUint::one()).is_err());
        }
    }

    #[test]
    fn one_whole_register_sample_closes_the_source_bound_factor_pair() {
        let mut executor =
            WholeRegisterPhaseExecutor::prepare(BigUint::from(15u8), BigUint::from(2u8), Some(4))
                .unwrap();
        let shot = executor
            .measure(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(shot.phase.numerator, BigUint::from(4u8));
        let closure = shot.closure.expect("phase winding closes N=15");
        let (p, q) = closure.factors();
        assert_eq!(p * q, BigUint::from(15u8));
    }

    #[test]
    fn terminal_readout_can_stop_before_factor_closure() {
        let mut executor =
            WholeRegisterPhaseExecutor::prepare(BigUint::from(15u8), BigUint::from(2u8), Some(4))
                .unwrap();
        let mut observed = None;
        let result =
            executor.measure_with_readout(&BigUint::from(5u8), &BigUint::from(8u8), |phase| {
                observed = Some(phase.clone());
                Err("terminal port consumed the phase".into())
            });
        assert!(matches!(result, Err(error) if error == "terminal port consumed the phase"));
        let phase = observed.unwrap();
        assert_eq!(phase.numerator, BigUint::from(4u8));
        assert_eq!(phase.denominator, BigUint::from(16u8));
    }

    #[test]
    fn prepared_rewirer_drives_the_single_full_register_sample() {
        let source = BigUint::from(15u8);
        let base = BigUint::from(2u8);
        let work = prepared_work(&source, &base, 1);
        let mut circuit = WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(
            source,
            base,
            4,
            &[],
            Some(&work),
        )
        .unwrap();
        let readout = circuit
            .fire(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(readout.numerator, BigUint::from(4u8));
        assert_eq!(readout.denominator, BigUint::from(16u8));
    }

    #[test]
    fn y_reentry_orientation_is_applied_before_the_joint_readout() {
        let mut executor = WholeRegisterPhaseExecutor::prepare_with_reentry(
            BigUint::from(15u8),
            BigUint::from(2u8),
            Some(4),
            1729,
            &[(true, false), (false, true)],
        )
        .unwrap();
        let shot = executor
            .measure(&BigUint::from(5u8), &BigUint::from(8u8))
            .unwrap();
        assert_eq!(shot.phase.numerator, BigUint::from(4u8));
        assert!(shot.closure.is_some());
        assert!(WholeRegisterPhaseExecutor::prepare_with_reentry(
            BigUint::from(15u8),
            BigUint::from(2u8),
            Some(4),
            1729,
            &[(true, false)],
        )
        .is_err());
    }
}
