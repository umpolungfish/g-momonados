//! Source-bound recycled phase circuit, without a residue amplitude register.
//!
//! A carrier must actually apply the emitted gates and supply its readout.
//! This adapter does not implement that device or substitute phase samples.
use crate::phase_unbraid::PhaseReadoutAccumulator;
use crate::reversible_modular::{ElementaryGate, ModularMultiply};
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use vox_core::fixed_point_quantum_phase::execution::{Executor, Program};
use vox_core::vox::{EVALF, EVALT};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CarrierGate {
    Reversible(ElementaryGate),
    Hadamard(usize),
    /// diag(1, exp(-2 pi i numerator / 2^denominator_bits)).
    /// Keep the complete dyadic coordinate until calibrated braid lowering.
    Feedback {
        qubit: usize,
        numerator: BigUint,
        denominator_bits: usize,
    },
}

/// Targets for calibrated braid synthesis. No Toffoli or multi-control gate
/// remains after this lowering; feedback retains its complete dyadic phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BraidTarget {
    X(usize),
    H(usize),
    T {
        qubit: usize,
        inverse: bool,
    },
    Cnot {
        control: usize,
        target: usize,
    },
    Feedback {
        qubit: usize,
        numerator: BigUint,
        denominator_bits: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkPreparation {
    /// Prepare each valid residue `0 <= x < N` with probability `1/N`.
    UniformResidues,
}

impl CarrierGate {
    /// Stream an exact Clifford+T decomposition. In particular, Toffoli is
    /// lowered with its full phase corrections, not a relative-phase variant.
    /// Reference decomposition: Qiskit's standard CCX definition,
    /// https://github.com/Qiskit/qiskit/blob/main/qiskit/circuit/library/standard_gates/x.py
    pub fn lower_for_braid<F>(&self, mut emit: F) -> Result<(), String>
    where
        F: FnMut(BraidTarget) -> Result<(), String>,
    {
        match self {
            Self::Hadamard(q) => emit(BraidTarget::H(*q)),
            Self::Feedback {
                qubit,
                numerator,
                denominator_bits,
            } => emit(BraidTarget::Feedback {
                qubit: *qubit,
                numerator: numerator.clone(),
                denominator_bits: *denominator_bits,
            }),
            Self::Reversible(ElementaryGate::X(q)) => emit(BraidTarget::X(*q)),
            Self::Reversible(ElementaryGate::Cnot { control, target }) => {
                if control == target {
                    return Err("CNOT control overlaps target".into());
                }
                emit(BraidTarget::Cnot {
                    control: *control,
                    target: *target,
                })
            }
            Self::Reversible(ElementaryGate::Toffoli {
                first,
                second,
                target,
            }) => {
                let (a, b, t) = (*first, *second, *target);
                if a == b || a == t || b == t {
                    return Err("Toffoli wires overlap".into());
                }
                let h = |q| BraidTarget::H(q);
                let cx = |control, target| BraidTarget::Cnot { control, target };
                let phase = |qubit, inverse| BraidTarget::T { qubit, inverse };
                for operation in [
                    h(t),
                    cx(b, t),
                    phase(t, true),
                    cx(a, t),
                    phase(t, false),
                    cx(b, t),
                    phase(t, true),
                    cx(a, t),
                    phase(b, false),
                    phase(t, false),
                    h(t),
                    cx(a, b),
                    phase(a, false),
                    phase(b, true),
                    cx(a, b),
                ] {
                    emit(operation)?;
                }
                Ok(())
            }
        }
    }
}

/// Carrier execution boundary. `begin` prepares one pure control qubit in
/// `|0⟩`, the modular value register in the uniform mixture over `0 <= x < N`,
/// and the arithmetic scratch required by this gate layout in `|0⟩`. The
/// measured phase is therefore drawn from the modular orbit of a mixed-register
/// value, rather than from a fixed residue supplied by the executor.
pub trait Carrier {
    fn begin(
        &mut self,
        source: &[char],
        base: &[char],
        qubits: usize,
        phase_bits: usize,
        work_preparation: WorkPreparation,
    ) -> Result<(), String>;
    fn apply_target(&mut self, gate: BraidTarget) -> Result<(), String>;
    fn apply(&mut self, gate: CarrierGate) -> Result<(), String> {
        gate.lower_for_braid(|target| self.apply_target(target))
    }
    fn measure_control(&mut self) -> Result<bool, String>;
    fn finish(&mut self) -> Result<(), String>;
    /// Discard a partially executed shot after a gate or readout failure.
    fn abort(&mut self);
}

fn value(tape: &[char]) -> Result<BigUint, String> {
    if tape.is_empty() {
        return Err("empty source numeral".into());
    }
    let mut n = BigUint::zero();
    for (bit, &cell) in tape.iter().enumerate() {
        match cell {
            EVALF => n |= BigUint::one() << bit,
            EVALT => {}
            _ => return Err("source contains a non-numeral cell".into()),
        }
    }
    Ok(n)
}

fn coprime(mut a: BigUint, mut b: BigUint) -> bool {
    while !b.is_zero() {
        let next = &a % &b;
        a = b;
        b = next;
    }
    a.is_one()
}

/// Connect Vox's sealed source program to streamed reversible gate execution.
/// The detailed error remains available when Vox's static error interface is
/// used. There is no period search, fixture factor input or amplitude map.
pub struct RecycledCarrierExecutor<C> {
    carrier: C,
    last_error: Option<String>,
    phase_evidence: PhaseReadoutAccumulator,
}

/// One completed carrier shot and its possible algebraic factor closure.
/// Absence of factors is a valid readout outcome, not a synthesized result.
pub struct FactorShot {
    pub phase: BigUint,
    pub phase_bits: usize,
    pub closure: Option<(BigUint, BigUint, BigUint)>,
    /// The two factor arms remain paired with their source and phase order
    /// after closure, so later inverse/readout steps cannot clear one arm.
    pub result_pair: Option<ClosedFactorPair>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedFactorPair {
    pub source: BigUint,
    pub base: BigUint,
    pub order: BigUint,
    pub p: BigUint,
    pub q: BigUint,
}

impl ClosedFactorPair {
    fn close(
        source: &BigUint,
        base: &BigUint,
        closure: &(BigUint, BigUint, BigUint),
    ) -> Result<Self, String> {
        let (order, p, q) = closure;
        if p <= &BigUint::one() || q <= &BigUint::one() || p * q != *source {
            return Err("factor arms did not fuse to their source".into());
        }
        Ok(Self {
            source: source.clone(),
            base: base.clone(),
            order: order.clone(),
            p: p.clone(),
            q: q.clone(),
        })
    }
}

impl<C: Carrier> RecycledCarrierExecutor<C> {
    pub fn new(carrier: C) -> Self {
        Self {
            carrier,
            last_error: None,
            phase_evidence: PhaseReadoutAccumulator::default(),
        }
    }
    pub fn carrier(&self) -> &C {
        &self.carrier
    }
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Execute a shot, then close only its measured phase against its sealed
    /// source and base. Evidence is accumulated across matching modular orbits.
    pub fn execute_factor_shot(&mut self, program: &Program) -> Result<FactorShot, String> {
        let result: Result<FactorShot, String> = (|| {
            let tape = self.execute_program(program)?;
            if tape.len() != program.phase_width() {
                return Err("carrier phase tape differs from program width".into());
            }
            let phase = value(&tape)?;
            let n = value(program.n())?;
            let base = value(program.base())?;
            let width = u64::try_from(program.phase_width())
                .map_err(|_| "phase width exceeds closure indexing")?;
            let closure = self.phase_evidence.close(&phase, width, &base, &n)?;
            let result_pair = closure.as_ref()
                .map(|pair| ClosedFactorPair::close(&n, &base, pair))
                .transpose()?;
            Ok(FactorShot {
                phase,
                phase_bits: program.phase_width(),
                closure,
                result_pair,
            })
        })();
        if let Err(error) = &result {
            self.last_error = Some(error.clone());
        }
        result
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<Vec<char>, String> {
        self.last_error = None;
        let n = value(program.n())?;
        let base = value(program.base())?;
        let arithmetic = ModularMultiply::new(&n)?;
        if !coprime(base.clone(), n.clone()) {
            return Err("phase base is not coprime to source".into());
        }
        let phase_bits = program.phase_width();
        if phase_bits
            < arithmetic
                .width()
                .checked_mul(2)
                .ok_or("phase width overflow")?
        {
            return Err("source program has insufficient phase precision".into());
        }
        phase_bits.checked_add(1).ok_or("feedback width overflow")?;
        // These are classical gate constants, not modular orbit states.
        let mut powers = Vec::with_capacity(phase_bits);
        let mut power = base % &n;
        for _ in 0..phase_bits {
            powers.push(power.clone());
            power = (&power * &power) % &n;
        }
        if let Err(error) = self.carrier.begin(
            program.n(),
            program.base(),
            arithmetic.elementary_qubits(),
            phase_bits,
            WorkPreparation::UniformResidues,
        ) {
            self.carrier.abort();
            return Err(error);
        }
        let result = self.execute_shot(&arithmetic, &powers);
        match result {
            Ok(tape) => match self.carrier.finish() {
                Ok(()) => Ok(tape),
                Err(error) => {
                    self.carrier.abort();
                    Err(error)
                }
            },
            Err(error) => {
                self.carrier.abort();
                Err(error)
            }
        }
    }

    fn execute_shot(
        &mut self,
        arithmetic: &ModularMultiply,
        powers: &[BigUint],
    ) -> Result<Vec<char>, String> {
        let mut readout = BigUint::zero();
        let mut tape = Vec::with_capacity(powers.len());
        for (bit_index, multiplier) in powers.iter().rev().enumerate() {
            self.carrier.apply(CarrierGate::Hadamard(0))?;
            arithmetic.emit_elementary(multiplier, |gate| {
                self.carrier.apply(CarrierGate::Reversible(gate))
            })?;
            if !readout.is_zero() {
                self.carrier.apply(CarrierGate::Feedback {
                    qubit: 0,
                    numerator: readout.clone(),
                    denominator_bits: bit_index + 1,
                })?;
            }
            self.carrier.apply(CarrierGate::Hadamard(0))?;
            let bit = self.carrier.measure_control()?;
            tape.push(if bit { EVALF } else { EVALT });
            if bit {
                readout |= BigUint::one() << bit_index;
                // Recycle the measured control while retaining the work state.
                self.carrier
                    .apply(CarrierGate::Reversible(ElementaryGate::X(0)))?;
            }
        }
        Ok(tape)
    }
}

impl<C: Carrier> Executor for RecycledCarrierExecutor<C> {
    fn execute_and_measure(&mut self, program: &Program) -> Result<Vec<char>, &'static str> {
        match self.execute_program(program) {
            Ok(tape) => Ok(tape),
            Err(error) => {
                self.last_error = Some(error);
                Err("recycled carrier execution failed; inspect last_error")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use vox_core::fixed_point_quantum_membrane::FixedPointQuantumMembrane;

    #[test]
    fn closed_factor_pair_banks_both_128_bit_semiprime_arms() {
        let source = BigUint::parse_bytes(
            b"296650821743515430283258444261036507151",
            10,
        )
        .unwrap();
        let p = BigUint::parse_bytes(b"16925480323643806501", 10).unwrap();
        let q = BigUint::parse_bytes(b"17526877587580975651", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let closure = (BigUint::from(16u8), p.clone(), q.clone());
        let pair = ClosedFactorPair::close(&source, &BigUint::from(2u8), &closure).unwrap();
        assert_eq!(pair.source, source);
        assert_eq!(pair.p, p);
        assert_eq!(pair.q, q);
        assert_eq!(pair.p.clone() * &pair.q, pair.source);
        assert!(ClosedFactorPair::close(
            &(pair.source + BigUint::one()),
            &pair.base,
            &closure,
        )
        .is_err());
    }

    // Exact Z[zeta_8] coefficients with zeta_8^4 = -1. H contributes
    // (zeta_8 - zeta_8^3)/2, so no floating point or rounded phase is used.
    fn root_shift(value: &[BigInt; 4], power: usize) -> [BigInt; 4] {
        let mut out: [BigInt; 4] = core::array::from_fn(|_| BigInt::zero());
        for (i, coefficient) in value.iter().enumerate() {
            let exponent = i + power;
            if (exponent / 4) % 2 == 0 {
                out[exponent % 4] += coefficient;
            } else {
                out[exponent % 4] -= coefficient;
            }
        }
        out
    }
    fn sqrt_two(value: &[BigInt; 4]) -> [BigInt; 4] {
        let first = root_shift(value, 1);
        let third = root_shift(value, 3);
        core::array::from_fn(|i| &first[i] - &third[i])
    }

    #[test]
    fn recycled_carrier_toffoli_phase_on_128_bit_semiprime_layout() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let layout = ModularMultiply::new(&n).unwrap();
        let wires = [1, 2, layout.width()];
        let gate = CarrierGate::Reversible(ElementaryGate::Toffoli {
            first: wires[0],
            second: wires[1],
            target: wires[2],
        });
        // Check the local operator at three wires of the full 128-bit source
        // layout. The other wires are spectators, not smaller factoring inputs.
        for basis in 0usize..8 {
            let mut state: [[BigInt; 4]; 8] =
                core::array::from_fn(|_| core::array::from_fn(|_| BigInt::zero()));
            state[basis][0] = BigInt::one();
            let mut denominator = BigInt::one();
            let wire = |q| wires.iter().position(|&w| w == q).unwrap();
            gate.lower_for_braid(|operation| {
                match operation {
                    BraidTarget::H(q) => {
                        let mask = 1 << wire(q);
                        for i in 0usize..8 {
                            if i & mask != 0 {
                                continue;
                            }
                            let sum = core::array::from_fn(|j| &state[i][j] + &state[i | mask][j]);
                            let difference =
                                core::array::from_fn(|j| &state[i][j] - &state[i | mask][j]);
                            state[i] = sqrt_two(&sum);
                            state[i | mask] = sqrt_two(&difference);
                        }
                        denominator *= 2;
                    }
                    BraidTarget::T { qubit, inverse } => {
                        let mask = 1 << wire(qubit);
                        for (i, value) in state.iter_mut().enumerate() {
                            if i & mask != 0 {
                                *value = root_shift(value, if inverse { 7 } else { 1 });
                            }
                        }
                    }
                    BraidTarget::Cnot { control, target } => {
                        let control_mask = 1 << wire(control);
                        let target_mask = 1 << wire(target);
                        for i in 0usize..8 {
                            if i & control_mask != 0 && i & target_mask == 0 {
                                state.swap(i, i | target_mask);
                            }
                        }
                    }
                    _ => panic!("unexpected Toffoli decomposition target"),
                }
                Ok(())
            })
            .unwrap();
            let expected = if basis & 3 == 3 { basis ^ 4 } else { basis };
            for (i, value) in state.iter().enumerate() {
                assert_eq!(
                    value[0],
                    if i == expected {
                        denominator.clone()
                    } else {
                        BigInt::zero()
                    }
                );
                assert!(value[1..].iter().all(Zero::is_zero));
            }
        }
    }

    struct UnavailableCarrier {
        source: Vec<char>,
        started: bool,
        aborted: bool,
        calls: usize,
    }
    impl Carrier for UnavailableCarrier {
        fn begin(
            &mut self,
            source: &[char],
            base: &[char],
            qubits: usize,
            phase_bits: usize,
            work_preparation: WorkPreparation,
        ) -> Result<(), String> {
            assert_eq!(source, self.source);
            assert_eq!(value(source)?.bits(), 128);
            assert_eq!(value(base)?, BigUint::from(2u8));
            assert_eq!(qubits, 388);
            assert_eq!(phase_bits, 256);
            assert_eq!(work_preparation, WorkPreparation::UniformResidues);
            self.started = true;
            Ok(())
        }
        fn apply_target(&mut self, operation: BraidTarget) -> Result<(), String> {
            self.calls += 1;
            assert_eq!(operation, BraidTarget::H(0));
            Err("calibrated carrier unavailable".into())
        }
        fn measure_control(&mut self) -> Result<bool, String> {
            panic!("failed carrier must not reach phase measurement");
        }
        fn finish(&mut self) -> Result<(), String> {
            panic!("failed carrier must not finish successfully");
        }
        fn abort(&mut self) {
            self.aborted = true;
        }
    }

    #[test]
    fn recycled_carrier_failure_preserves_128_bit_semiprime_binding() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let source: Vec<_> = (0..n.bits())
            .map(|bit| if n.bit(bit) { EVALF } else { EVALT })
            .collect();
        let program = FixedPointQuantumMembrane::from_n(&source)
            .unwrap()
            .prepare_structural_execution()
            .unwrap();
        let mut executor = RecycledCarrierExecutor::new(UnavailableCarrier {
            source,
            started: false,
            aborted: false,
            calls: 0,
        });
        assert!(executor.execute_and_measure(&program).is_err());
        assert_eq!(
            executor.last_error(),
            Some("calibrated carrier unavailable")
        );
        assert!(executor.carrier().started);
        assert!(executor.carrier().aborted);
        assert_eq!(executor.carrier().calls, 1);
        let mut factor_executor = RecycledCarrierExecutor::new(UnavailableCarrier {
            source: executor.carrier().source.clone(),
            started: false,
            aborted: false,
            calls: 0,
        });
        assert!(factor_executor.execute_factor_shot(&program).is_err());
        assert_eq!(factor_executor.last_error(), Some("calibrated carrier unavailable"));
        assert!(factor_executor.carrier().aborted);
    }

    struct RecordingCarrier {
        targets: Vec<BraidTarget>,
    }
    impl Carrier for RecordingCarrier {
        fn begin(
            &mut self,
            _source: &[char],
            _base: &[char],
            _qubits: usize,
            _phase_bits: usize,
            _work_preparation: WorkPreparation,
        ) -> Result<(), String> {
            Ok(())
        }
        fn apply_target(&mut self, target: BraidTarget) -> Result<(), String> {
            self.targets.push(target);
            Ok(())
        }
        fn measure_control(&mut self) -> Result<bool, String> {
            Ok(false)
        }
        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn abort(&mut self) {}
    }

    #[test]
    fn mixed_work_phase_shot_does_not_seed_a_fixed_residue_on_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let arithmetic = ModularMultiply::new(&n).unwrap();
        let carrier = RecordingCarrier { targets: Vec::new() };
        let mut executor = RecycledCarrierExecutor::new(carrier);
        let phase = executor
            .execute_shot(&arithmetic, &[BigUint::one()])
            .unwrap();
        assert_eq!(value(&phase).unwrap(), BigUint::zero());
        assert_eq!(
            executor.carrier().targets,
            [BraidTarget::H(0), BraidTarget::H(0)]
        );
    }
}
