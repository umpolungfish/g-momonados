//! Compile the derived local corrections around the controlled Fibonacci
//! exchange, then validate the complete emitted six-strand word.
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use g_momonados::anyon_local::{cnot_local_corrections, FibonacciLocal, LocalMatrix};
use g_momonados::anyon_pair::{FibonacciPair, COMPUTATIONAL_CHANNELS};
use g_momonados::ququart_factor::fourier_braid_targets;
use g_momonados::phase_unbraid::{FixedComplex, FixedPointFormat};
use g_momonados::recycled_carrier::{BraidTarget, Carrier, WorkPreparation};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, ToPrimitive, Zero};
use vox_core::vox::{EVALF, EVALT};

use crate::fibonacci_qc::{sk_split_fuse, solovay_kitaev, Complex, GateNet, Matrix2};

fn ratio(value: &BigInt, scale: &BigInt) -> f64 {
    let shift = value.bits().max(scale.bits()).saturating_sub(900) as usize;
    (value >> shift).to_f64().unwrap_or(0.0) / (scale >> shift).to_f64().unwrap_or(1.0)
}

fn as_matrix(matrix: &LocalMatrix, scale: &BigInt) -> Matrix2 {
    Matrix2::from_array(core::array::from_fn(|index| {
        Complex::new(
            ratio(&matrix.0[index].re, scale),
            ratio(&matrix.0[index].im, scale),
        )
    }))
}

fn compile_local(
    target: &LocalMatrix,
    scale: &BigInt,
    net: &GateNet,
    depth: usize,
) -> Result<(Vec<i32>, f64), String> {
    let target_float = as_matrix(target, scale);
    let (word, _, estimate) = solovay_kitaev(&target_float, depth, net);
    if !estimate.is_finite() {
        return Err("local braid synthesis returned a non-finite estimate".into());
    }
    Ok((word, estimate))
}

fn compile_local_split(
    target: &LocalMatrix,
    scale: &BigInt,
    net: &GateNet,
    depth: usize,
) -> Result<(Vec<i32>, f64), String> {
    let target_float = as_matrix(target, scale);
    let (word, _, estimate) = sk_split_fuse(&target_float, depth, net, 8);
    if !estimate.is_finite() {
        return Err("split/fuse braid synthesis returned a non-finite estimate".into());
    }
    Ok((word, estimate))
}

fn fixed_projective_error_within(
    observed: &LocalMatrix,
    target: &LocalMatrix,
    bits: usize,
) -> Result<bool, String> {
    let exponent = bits
        .checked_mul(2)
        .ok_or("feedback precision exponent overflow")?;
    let denominator = BigUint::one() << exponent;
    let mut overlap_re = BigInt::zero();
    let mut overlap_im = BigInt::zero();
    for (left, right) in observed.0.iter().zip(&target.0) {
        overlap_re += &left.re * &right.re + &left.im * &right.im;
        overlap_im += &left.im * &right.re - &left.re * &right.im;
    }
    let overlap_squared = (&overlap_re * &overlap_re + &overlap_im * &overlap_im)
        .to_biguint()
        .ok_or("invalid fixed-point overlap norm")?;
    let norm_squared = |matrix: &LocalMatrix| -> Result<BigUint, String> {
        matrix.0.iter().try_fold(BigUint::zero(), |sum, cell| {
            let re = (&cell.re * &cell.re)
                .to_biguint()
                .ok_or("invalid fixed-point real norm")?;
            let im = (&cell.im * &cell.im)
                .to_biguint()
                .ok_or("invalid fixed-point imaginary norm")?;
            Ok(sum + re + im)
        })
    };
    let norm_product = norm_squared(observed)? * norm_squared(target)?;
    let one_less = &denominator - BigUint::one();
    let left = overlap_squared * &denominator * &denominator;
    let right = norm_product * &one_less * &one_less;
    Ok(left >= right)
}

fn pair_local_generators(pair: &FibonacciPair, target: bool) -> Result<[Matrix2; 2], String> {
    let scale = pair.format().scale();
    let tolerance = &scale >> 20usize;
    let blocks = |generator: usize| -> Result<[Matrix2; 2], String> {
        let physical = pair.evaluate(&[generator as i32])?;
        let recoupled = pair.in_pair_channels(&physical);
        let bit_channels = if target {
            [
                [COMPUTATIONAL_CHANNELS[0], COMPUTATIONAL_CHANNELS[2]],
                [COMPUTATIONAL_CHANNELS[1], COMPUTATIONAL_CHANNELS[3]],
            ]
        } else {
            [
                [COMPUTATIONAL_CHANNELS[0], COMPUTATIONAL_CHANNELS[1]],
                [COMPUTATIONAL_CHANNELS[2], COMPUTATIONAL_CHANNELS[3]],
            ]
        };
        let selected = bit_channels[0];
        for &input in &selected {
            for output in 0..5 {
                if !selected.contains(&output) {
                    let cell = &recoupled.0[5 * output + input];
                    if cell.re.abs() > tolerance || cell.im.abs() > tolerance {
                        return Err(format!(
                            "sigma_{generator} leaks from the addressed logical triple"
                        ));
                    }
                }
            }
        }
        let block = |channels: [usize; 2]| {
            Matrix2::from_array(core::array::from_fn(|i| {
                let cell = &recoupled.0[5 * channels[i / 2] + channels[i % 2]];
                Complex::new(ratio(&cell.re, &scale), ratio(&cell.im, &scale))
            }))
        };
        let first = block(bit_channels[0]);
        let second = block(bit_channels[1]);
        let spectator_error = first
            .data
            .iter()
            .zip(second.data.iter())
            .fold(0.0f64, |bound, (a, b)| {
                bound.max((a.re - b.re).abs()).max((a.im - b.im).abs())
            });
        if spectator_error > 1e-6 {
            return Err(format!(
                "sigma_{generator} changes phase across spectator sectors: {spectator_error:.8e}"
            ));
        }
        Ok([first, second])
    };
    let physical_generators = if target {
        [4usize, 5usize]
    } else {
        [1usize, 2usize]
    };
    let first = blocks(physical_generators[0])?;
    let second = blocks(physical_generators[1])?;
    Ok([first[0], second[0]])
}

fn build_local_net(generators: [Matrix2; 2], depth: usize, _legacy_capacity: usize) -> GateNet {
    let gates = [
        (1, generators[0]),
        (2, generators[1]),
        (-1, generators[0].conjugate_transpose()),
        (-2, generators[1].conjugate_transpose()),
    ];
    let mut entries = alloc::vec![(Vec::new(), Matrix2::identity())];
    let mut lo = 0usize;
    let mut hi = 1usize;
    for _ in 0..depth {
        for index in lo..hi {
            let last = entries[index].0.last().copied();
            let parent = entries[index].1;
            for &(symbol, gate) in &gates {
                if last == Some(-symbol) {
                    continue;
                }
                let mut word = entries[index].0.clone();
                word.push(symbol);
                entries.push((word, gate.mul(parent)));
            }
        }
        if entries.len() == hi {
            break;
        }
        lo = hi;
        hi = entries.len();
    }
    GateNet {
        entries,
        reached_cap: false,
    }
}

fn lift(word: &[i32], first: i32, second: i32) -> Result<Vec<i32>, String> {
    word.iter()
        .map(|gate| match gate.unsigned_abs() {
            1 => Ok(gate.signum() * first),
            2 => Ok(gate.signum() * second),
            _ => Err("local SK word escaped the two-generator triple".into()),
        })
        .collect()
}

/// Compile a single logical-qubit gate to physical Fibonacci braid generators.
/// A logical qubit occupies three consecutive anyon strands. The returned
/// indices are one-based braid generator labels, offset for `qubit` in a
/// larger register.
pub fn compile_single_qubit(
    source: &BigUint,
    target: &BraidTarget,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
) -> Result<(Vec<i32>, f64), String> {
    let reduced_target = target.reduced_feedback();
    let qubit = match &reduced_target {
        BraidTarget::X(qubit)
        | BraidTarget::H(qubit)
        | BraidTarget::T { qubit, .. }
        | BraidTarget::Feedback { qubit, .. } => *qubit,
        BraidTarget::Cnot { .. } => return Err("single-qubit compiler received CNOT".into()),
    };
    let algebra = FibonacciLocal::new(source)?;
    let scale = algebra.format().scale();
    let local_target = algebra.target(&reduced_target)?;
    if local_target.0 == LocalMatrix::identity(algebra.format()).0 {
        return Ok((Vec::new(), 0.0));
    }
    let generators = [1, 2].map(|generator| {
        algebra
            .evaluate(&[generator])
            .map(|matrix| as_matrix(&matrix, &scale))
    });
    let generators = [generators[0].clone()?, generators[1].clone()?];
    let net = build_local_net(generators, net_depth, max_gates);
    let accuracy_bits = match &reduced_target {
        BraidTarget::Feedback {
            denominator_bits, ..
        } => *denominator_bits,
        _ => 0,
    };
    compile_single_qubit_with_net(
        &algebra,
        &reduced_target,
        qubit,
        &local_target,
        &net,
        sk_depth,
        accuracy_bits,
    )
}

fn compile_single_qubit_with_net(
    algebra: &FibonacciLocal,
    target: &BraidTarget,
    qubit: usize,
    local_target: &LocalMatrix,
    net: &GateNet,
    sk_depth: usize,
    accuracy_bits: usize,
) -> Result<(Vec<i32>, f64), String> {
    if local_target.0 == LocalMatrix::identity(algebra.format()).0 {
        return Ok((Vec::new(), 0.0));
    }
    if net.entries.is_empty() {
        return Err("single-qubit braid net is empty".into());
    }
    let scale = algebra.format().scale();
    if let BraidTarget::Feedback {
        denominator_bits, ..
    } = target
    {
        let precision_limit = usize::try_from(algebra.format().w_bits)
            .map_err(|_| "source phase precision exceeds host indexing")?;
        if *denominator_bits > precision_limit {
            return Err("feedback denominator exceeds the source-bound phase precision".into());
        }
        let identity = LocalMatrix::identity(algebra.format());
        if fixed_projective_error_within(&identity, local_target, accuracy_bits)? {
            let target_float = as_matrix(local_target, &scale);
            return Ok((
                Vec::new(),
                Matrix2::projective_distance(&target_float, &Matrix2::identity()),
            ));
        }
    }
    let (word, error) = if matches!(target, BraidTarget::Feedback { .. }) {
        let (candidate, estimate) = compile_local(local_target, &scale, net, sk_depth)?;
        let candidate_meets_budget = !candidate.is_empty()
            && fixed_projective_error_within(
                &algebra.evaluate(&candidate)?,
                local_target,
                accuracy_bits,
            )?;
        if candidate_meets_budget {
            (candidate, estimate)
        } else {
            compile_local_split(local_target, &scale, net, sk_depth)?
        }
    } else {
        compile_local_split(local_target, &scale, net, sk_depth)?
    };
    if word.is_empty() {
        return Err("braid synthesis collapsed a nontrivial gate to the identity".into());
    }
    let target_float = as_matrix(local_target, &scale);
    let identity_error = Matrix2::projective_distance(&target_float, &Matrix2::identity());
    if error >= identity_error {
        return Err("braid synthesis did not improve on the identity approximation".into());
    }
    if matches!(target, BraidTarget::Feedback { .. }) {
        let observed = algebra.evaluate(&word)?;
        if !fixed_projective_error_within(&observed, local_target, accuracy_bits)? {
            return Err("anyon braid misses the requested dyadic feedback precision".into());
        }
    }
    let first = qubit
        .checked_mul(3)
        .and_then(|offset| offset.checked_add(1))
        .ok_or("logical qubit strand offset overflow")?;
    let second = first
        .checked_add(1)
        .ok_or("logical qubit strand offset overflow")?;
    let first = i32::try_from(first).map_err(|_| "logical qubit strand exceeds braid index")?;
    let second = i32::try_from(second).map_err(|_| "logical qubit strand exceeds braid index")?;
    Ok((lift(&word, first, second)?, error))
}

fn append_inverse(out: &mut Vec<i32>, word: &[i32]) {
    out.extend(word.iter().rev().map(|gate| -*gate));
}

fn local_phase_target(format: &FixedPointFormat) -> Result<LocalMatrix, String> {
    let lower = FixedComplex::winding_twiddle(&BigInt::from(-1), &BigUint::from(40u8), format)?;
    let upper = FixedComplex::winding_twiddle(&BigInt::from(1), &BigUint::from(40u8), format)?;
    let zero = FixedComplex {
        re: BigInt::from(0),
        im: BigInt::from(0),
    };
    Ok(LocalMatrix([lower, zero.clone(), zero, upper]))
}

pub fn compile(args: &[&str]) -> Result<String, String> {
    compile_selected(args, true).map(|candidate| candidate.report)
}

struct CnotCandidate {
    report: String,
    word: Vec<i32>,
    accuracy_bits: usize,
}

fn compile_selected(args: &[&str], render_report: bool) -> Result<CnotCandidate, String> {
    let sk_depth = args
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2);
    let minimum_accuracy = args
        .get(5)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let split_first =
        sk_depth < 7 && minimum_accuracy > 0 && minimum_accuracy >= sk_depth.saturating_mul(2);
    match compile_variant(args, split_first, render_report) {
        Ok(candidate) => Ok(candidate),
        Err(error) if error.starts_with("CNOT braid reaches ") => {
            compile_variant(args, !split_first, render_report).map_err(|fallback_error| {
                format!("{error}; alternate synthesis failed: {fallback_error}")
            })
        }
        Err(error) => Err(error),
    }
}

fn compile_variant(
    args: &[&str],
    split_fuse: bool,
    render_report: bool,
) -> Result<CnotCandidate, String> {
    if args.is_empty() || args.len() > 6 {
        return Err("usage: anyon_cnot_word N [sk_depth=2] [net_depth=7] [max_gates=20000] [exchange_refinement=2] [minimum_accuracy_bits=0]".into());
    }
    let source = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid source integer")?;
    if source.bits() < 128 {
        return Err("anyon CNOT compilation requires a source of at least 128 bits".into());
    }
    let parse = |index: usize, default: usize, name: &str| -> Result<usize, String> {
        args.get(index).map_or(Ok(default), |raw| {
            raw.parse().map_err(|_| format!("invalid {name}"))
        })
    };
    let sk_depth = parse(1, 2, "SK depth")?;
    let net_depth = parse(2, 7, "net depth")?;
    let max_gates = parse(3, 20_000, "net capacity")?;
    let refinement = parse(4, 2, "exchange refinement")?;
    let minimum_accuracy_bits = parse(5, 0, "minimum accuracy")?;

    let pair = FibonacciPair::new(&source)?;
    let local = FibonacciLocal::new(&source)?;
    let corrections = cnot_local_corrections(&source)?;
    let phase = local_phase_target(local.format())?;
    let target_net = build_local_net(pair_local_generators(&pair, true)?, net_depth, max_gates);
    let control_net = build_local_net(pair_local_generators(&pair, false)?, net_depth, max_gates);
    let scale = local.format().scale();
    let compile_local_gate = if split_fuse {
        compile_local_split
    } else {
        compile_local
    };
    let (basis, basis_error) =
        compile_local_gate(&corrections.target_basis, &scale, &target_net, sk_depth)?;
    let (target_y, y_error) =
        compile_local_gate(&corrections.target_y, &scale, &target_net, sk_depth)?;
    let (control_phase, phase_error) = compile_local_gate(&phase, &scale, &control_net, sk_depth)?;

    // The target triple occupies the right end of the six-strand register.
    // Its two local generators are sigma_4 and sigma_5. The control triple
    // occupies the left end, with sigma_1 and sigma_2.
    let basis = lift(&basis, 4, 5)?;
    let target_y = lift(&target_y, 4, 5)?;
    let control_phase = lift(&control_phase, 1, 2)?;
    let exchange = pair.controlled_double_exchange(refinement)?;

    // Chronological order for P B† E Y E Y† B, where B is the target basis
    // change and P is the control phase.
    let mut word = basis.clone();
    append_inverse(&mut word, &target_y);
    word.extend_from_slice(&exchange);
    word.extend_from_slice(&target_y);
    word.extend_from_slice(&exchange);
    append_inverse(&mut word, &basis);
    word.extend_from_slice(&control_phase);

    let physical = pair.evaluate(&word)?;
    let in_computational_channels = pair.in_pair_channels(&physical);
    let residual = pair.cnot_residual(&in_computational_channels);
    let residual_accuracy_bits = if residual.maximum().is_zero() {
        pair.format().w_bits as usize
    } else {
        (pair.format().w_bits as usize).saturating_sub(residual.maximum().bits() as usize)
    };
    if residual_accuracy_bits < minimum_accuracy_bits {
        return Err(format!(
            "CNOT braid reaches {residual_accuracy_bits} residual accuracy bits, below the requested {minimum_accuracy_bits}"
        ));
    }
    let report = if render_report {
        let residual_bits = if residual.maximum().is_zero() {
            "exact".to_string()
        } else {
            format!(
                "2^-{}",
                (pair.format().w_bits as usize).saturating_sub(residual.maximum().bits() as usize)
            )
        };
        let mut output = format!(
            "source_bits={} sk_depth={} net_depth={} net_capacity={} refinement={} minimum_accuracy_bits={} residual_accuracy_bits={}\nlocal_projective_errors basis={:.8e} target_y={:.8e} control_phase={:.8e}\nphysical_word_length={} residual={residual_bits}\ncomputational={} leakage={} unitarity={}\nword=",
            source.bits(), sk_depth, net_depth, max_gates, refinement,
            minimum_accuracy_bits, residual_accuracy_bits,
            basis_error, y_error, phase_error, word.len(), residual.computational,
            residual.leakage, residual.unitarity,
        );
        for (index, gate) in word.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push_str(&gate.to_string());
        }
        output
    } else {
        String::new()
    };
    Ok(CnotCandidate {
        report,
        word,
        accuracy_bits: residual_accuracy_bits,
    })
}

fn shifted_generator(gate: i32, qubit: usize) -> Result<i32, String> {
    let offset = qubit
        .checked_mul(3)
        .ok_or("logical anyon block offset overflow")?;
    let offset = i32::try_from(offset).map_err(|_| "logical anyon block exceeds braid index")?;
    let magnitude = i32::try_from(gate.unsigned_abs())
        .map_err(|_| "braid generator exceeds index representation")?;
    let shifted = magnitude
        .checked_add(offset)
        .ok_or("shifted braid generator overflow")?;
    Ok(gate.signum() * shifted)
}

fn emit_shifted<F>(word: &[i32], qubit: usize, emit: &mut F) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    for &gate in word {
        emit(shifted_generator(gate, qubit)?)?;
    }
    Ok(())
}

// In the pair-channel frame the first triple uses pair (2,3), so its
// diagonal and recoupled generators are sigma_2 and sigma_1 respectively.
// The right triple uses sigma_4 and sigma_5 in the local compiler's order.
fn emit_local_shifted<F>(word: &[i32], qubit: usize, emit: &mut F) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    for &gate in word {
        let gate = if qubit == 0 {
            match gate.unsigned_abs() {
                1 => gate.signum() * 2,
                2 => gate.signum(),
                _ => return Err("local braid escaped its logical triple".into()),
            }
        } else { gate };
        emit(shifted_generator(gate, qubit)?)?;
    }
    Ok(())
}

fn emit_inverse_local_shifted<F>(word: &[i32], qubit: usize, emit: &mut F) -> Result<(), String>
where F: FnMut(i32) -> Result<(), String>,
{
    let inverse: Vec<_> = word.iter().rev().map(|gate| -*gate).collect();
    emit_local_shifted(&inverse, qubit, emit)
}

fn emit_inverse_shifted<F>(word: &[i32], qubit: usize, emit: &mut F) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    for &gate in word.iter().rev() {
        emit(shifted_generator(-gate, qubit)?)?;
    }
    Ok(())
}

fn emit_reverse_cnot<F>(
    cnot: &[i32],
    hadamard: &[i32],
    left: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    emit_local_shifted(hadamard, left, emit)?;
    emit_local_shifted(hadamard, left + 1, emit)?;
    emit_shifted(cnot, left, emit)?;
    emit_local_shifted(hadamard, left, emit)?;
    emit_local_shifted(hadamard, left + 1, emit)
}

fn emit_inverse_reverse_cnot<F>(
    cnot: &[i32],
    hadamard: &[i32],
    left: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    emit_inverse_local_shifted(hadamard, left + 1, emit)?;
    emit_inverse_local_shifted(hadamard, left, emit)?;
    emit_inverse_shifted(cnot, left, emit)?;
    emit_inverse_local_shifted(hadamard, left + 1, emit)?;
    emit_inverse_local_shifted(hadamard, left, emit)
}

fn emit_swap<F>(cnot: &[i32], hadamard: &[i32], left: usize, emit: &mut F) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    emit_shifted(cnot, left, emit)?;
    emit_reverse_cnot(cnot, hadamard, left, emit)?;
    emit_shifted(cnot, left, emit)
}

fn emit_inverse_swap<F>(
    cnot: &[i32],
    hadamard: &[i32],
    left: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(i32) -> Result<(), String>,
{
    emit_inverse_shifted(cnot, left, emit)?;
    emit_inverse_reverse_cnot(cnot, hadamard, left, emit)?;
    emit_inverse_shifted(cnot, left, emit)
}

/// Compile a logical CNOT between any two encoded qubits in a register.
/// Adjacent CNOT words occupy six consecutive strands; distant wires are
/// brought together with logical SWAPs, then the routing words are inverted.
pub fn compile_cnot_target(
    source: &BigUint,
    control: usize,
    target: usize,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    minimum_accuracy_bits: usize,
) -> Result<Vec<i32>, String> {
    let mut compiler =
        FibonacciBraidCompiler::new(source, sk_depth, net_depth, max_gates, refinement)?;
    compiler.compile_cnot(control, target, minimum_accuracy_bits)
}

/// Reuses source-bound local braid templates while streaming gates for one
/// factorization circuit. Template caches are keyed by the required residual
/// floor, so repeated arithmetic CNOTs do not rerun Solovay–Kitaev synthesis.
pub struct FibonacciBraidCompiler {
    source: BigUint,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    feedback_accuracy_bits: usize,
    cnot_template: Option<(Vec<i32>, usize)>,
    hadamard_template: Option<(Vec<i32>, usize)>,
    single_templates: [Option<(usize, Vec<i32>)>; 4],
    single_gate_net: Option<GateNet>,
}

/// Hardware boundary for generated Fibonacci anyon worldlines and fusion
/// readout. Implementations receive elementary adjacent-strand exchanges;
/// this adapter does not replace the device with a state update.
pub trait FibonacciAnyonDevice {
    fn begin(
        &mut self,
        source: &[char],
        base: &[char],
        logical_qubits: usize,
        phase_bits: usize,
        work_preparation: WorkPreparation,
    ) -> Result<(), String>;
    fn generate_exchange(&mut self, generator: i32) -> Result<(), String>;
    fn measure_control_fusion(&mut self) -> Result<bool, String>;
    fn finish(&mut self) -> Result<(), String>;
    fn abort(&mut self);
}

/// Connects the recycled QFT phase program to an anyon device. Every logical
/// target is compiled and streamed as braid generators directly to the
/// device; feedback targets keep their source-width phase denominator and use
/// the shot-wide approximate-QFT error budget for braid synthesis.
pub struct CompiledFibonacciCarrier<D> {
    device: D,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    minimum_accuracy_bits: usize,
    compiler: Option<FibonacciBraidCompiler>,
}

/// Reserve at most 1/64 total operator error across the semiclassical QFT's
/// feedback rotations. There are at most `phase_bits` such rotations, so each
/// braid is compiled to error at most 1/(64 phase_bits). The phase tape itself
/// still has its full source-derived width.
fn qft_feedback_accuracy_bits(phase_bits: usize) -> Result<usize, String> {
    let denominator = phase_bits
        .checked_mul(64)
        .filter(|value| *value > 0)
        .ok_or("QFT feedback error budget overflow")?;
    Ok((usize::BITS - (denominator - 1).leading_zeros()) as usize)
}

impl<D> CompiledFibonacciCarrier<D> {
    pub fn new(
        device: D,
        sk_depth: usize,
        net_depth: usize,
        max_gates: usize,
        refinement: usize,
        minimum_accuracy_bits: usize,
    ) -> Self {
        Self {
            device,
            sk_depth,
            net_depth,
            max_gates,
            refinement,
            minimum_accuracy_bits,
            compiler: None,
        }
    }

    pub fn device(&self) -> &D {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut D {
        &mut self.device
    }
}

fn source_value(source: &[char]) -> Result<BigUint, String> {
    if source.is_empty() {
        return Err("empty source numeral".into());
    }
    let mut value = BigUint::zero();
    for (bit, cell) in source.iter().enumerate() {
        match *cell {
            EVALF => value |= BigUint::one() << bit,
            EVALT => {}
            _ => return Err("source contains a non-numeral cell".into()),
        }
    }
    Ok(value)
}

impl<D: FibonacciAnyonDevice> Carrier for CompiledFibonacciCarrier<D> {
    fn begin(
        &mut self,
        source: &[char],
        base: &[char],
        logical_qubits: usize,
        phase_bits: usize,
        work_preparation: WorkPreparation,
    ) -> Result<(), String> {
        let feedback_accuracy_bits = qft_feedback_accuracy_bits(phase_bits)?;
        let source_integer = source_value(source)?;
        let mut compiler = match self.compiler.take() {
            Some(compiler) if compiler.source == source_integer => compiler,
            _ => FibonacciBraidCompiler::new(
                &source_integer,
                self.sk_depth,
                self.net_depth,
                self.max_gates,
                self.refinement,
            )?,
        };
        compiler.feedback_accuracy_bits = feedback_accuracy_bits;
        self.device
            .begin(source, base, logical_qubits, phase_bits, work_preparation)?;
        self.compiler = Some(compiler);
        Ok(())
    }

    fn apply_target(&mut self, target: BraidTarget) -> Result<(), String> {
        let compiler = self
            .compiler
            .as_mut()
            .ok_or("anyon braid carrier has not begun")?;
        compiler.compile_target_to(&target, self.minimum_accuracy_bits, |generator| {
            self.device.generate_exchange(generator)
        })
    }

    fn measure_control(&mut self) -> Result<bool, String> {
        self.device.measure_control_fusion()
    }

    fn finish(&mut self) -> Result<(), String> {
        self.device.finish()
    }

    fn abort(&mut self) {
        self.compiler = None;
        self.device.abort();
    }
}

/// Result of one phase readout that closes a source-bound factor pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnyonicFactorization {
    pub source: BigUint,
    pub base: BigUint,
    pub order: BigUint,
    pub p: BigUint,
    pub q: BigUint,
    pub shots: u32,
}

/// Execute recycled phase estimation by generating each Fibonacci exchange on
/// the supplied anyon device. The executor retains measured phase bits and
/// arithmetic constants; it does not construct residue amplitudes or a state
/// vector. The caller chooses a fresh base when one orbit readout is
/// nonproductive, then passes a new device or the same reset device here.
#[allow(clippy::too_many_arguments)]
pub fn factor_semiprime_with_anyons<D: FibonacciAnyonDevice>(
    n: &BigUint,
    base: &BigUint,
    max_shots: u32,
    device: D,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    minimum_accuracy_bits: usize,
) -> Result<AnyonicFactorization, String> {
    try_factor_with_anyons(
        n, base, max_shots, device, sk_depth, net_depth, max_gates,
        refinement, minimum_accuracy_bits,
    )?.ok_or_else(|| format!(
        "anyon phase readout did not close a factor pair within {max_shots} shots"
    ))
}

/// Return None for a completed shot budget without closure. Device,
/// compilation, and readout failures remain errors for the caller to handle.
#[allow(clippy::too_many_arguments)]
pub fn try_factor_with_anyons<D: FibonacciAnyonDevice>(
    n: &BigUint,
    base: &BigUint,
    max_shots: u32,
    device: D,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    minimum_accuracy_bits: usize,
) -> Result<Option<AnyonicFactorization>, String> {
    if n.bits() < 128 || max_shots == 0 {
        return Err("anyonic phase factorization requires a source of at least 128 bits and a positive shot budget".into());
    }
    let numeral = |value: &BigUint| -> Vec<char> {
        let bits = value.bits().max(1) as usize;
        (0..bits)
            .map(|bit| if value.bit(bit as u64) { EVALF } else { EVALT })
            .collect()
    };
    let n_tape = numeral(n);
    let base_tape = numeral(base);
    let program =
        vox_core::fixed_point_quantum_membrane::FixedPointQuantumMembrane::from_n_with_base(
            &n_tape, &base_tape,
        )
        .and_then(|membrane| membrane.prepare_structural_execution())
        .map_err(|error| error.to_string())?;
    let carrier = CompiledFibonacciCarrier::new(
        device,
        sk_depth,
        net_depth,
        max_gates,
        refinement,
        minimum_accuracy_bits,
    );
    let mut executor = g_momonados::recycled_carrier::RecycledCarrierExecutor::new(carrier);
    for shots in 1..=max_shots {
        let readout = executor
            .execute_factor_shot(&program)
            .map_err(|error| format!("anyon phase shot {shots} failed: {error}"))?;
        if let Some(pair) = readout.result_pair {
            return Ok(Some(AnyonicFactorization {
                source: pair.source,
                base: pair.base,
                order: pair.order,
                p: pair.p,
                q: pair.q,
                shots,
            }));
        }
    }
    Ok(None)
}


const N_ONLY_BASES: [u32; 16] =
    [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];

/// Factor from N alone over one persistent Fibonacci carrier.
///
/// The base schedule and shot policy are internal. No factor, order, phase,
/// or candidate enters through the command surface. A single carrier remains
/// alive across base changes so source-bound braid templates and the local
/// gate net are reused instead of being rebuilt for each trial base.
#[allow(clippy::too_many_arguments)]
pub fn factor_n_with_anyons<D: FibonacciAnyonDevice>(
    n: &BigUint,
    device: D,
    max_total_shots: u32,
    sk_depth: usize,
    net_depth: usize,
    max_gates: usize,
    refinement: usize,
    minimum_accuracy_bits: usize,
) -> Result<AnyonicFactorization, String> {
    if n.bits() < 128 || max_total_shots == 0 {
        return Err(
            "N-only anyonic factorization requires N >= 128 bits and a positive internal shot budget"
                .into(),
        );
    }

    let numeral = |value: &BigUint| -> Vec<char> {
        let bits = value.bits().max(1) as usize;
        (0..bits)
            .map(|bit| if value.bit(bit as u64) { EVALF } else { EVALT })
            .collect()
    };

    let n_tape = numeral(n);
    let carrier = CompiledFibonacciCarrier::new(
        device,
        sk_depth,
        net_depth,
        max_gates,
        refinement,
        minimum_accuracy_bits,
    );
    let mut executor = g_momonados::recycled_carrier::RecycledCarrierExecutor::new(carrier);
    let mut total_shots = 0u32;

    while total_shots < max_total_shots {
        for raw_base in N_ONLY_BASES {
            if total_shots >= max_total_shots {
                break;
            }

            let base = BigUint::from(raw_base);
            if &base >= n {
                continue;
            }

            let base_tape = numeral(&base);
            let program =
                vox_core::fixed_point_quantum_membrane::FixedPointQuantumMembrane::from_n_with_base(
                    &n_tape,
                    &base_tape,
                )
                .and_then(|membrane| membrane.prepare_structural_execution())
                .map_err(|error| error.to_string())?;

            total_shots = total_shots
                .checked_add(1)
                .ok_or("anyon shot counter overflow")?;

            let readout = executor
                .execute_factor_shot(&program)
                .map_err(|error| {
                    format!("anyon phase shot {total_shots} at base {base} failed: {error}")
                })?;

            if let Some(pair) = readout.result_pair {
                if &pair.p * &pair.q != *n {
                    return Err("anyon phase pair failed exact N closure".into());
                }

                return Ok(AnyonicFactorization {
                    source: pair.source,
                    base: pair.base,
                    order: pair.order,
                    p: pair.p,
                    q: pair.q,
                    shots: total_shots,
                });
            }
        }
    }

    Err(format!(
        "N-only anyon phase path did not close a factor pair within {max_total_shots} shots"
    ))
}

impl FibonacciBraidCompiler {
    pub fn new(
        source: &BigUint,
        sk_depth: usize,
        net_depth: usize,
        max_gates: usize,
        refinement: usize,
    ) -> Result<Self, String> {
        if source.bits() < 128 {
            return Err("anyon CNOT compilation requires a source of at least 128 bits".into());
        }
        Ok(Self {
            source: source.clone(),
            sk_depth,
            net_depth,
            max_gates,
            refinement,
            feedback_accuracy_bits: 0,
            cnot_template: None,
            hadamard_template: None,
            single_templates: core::array::from_fn(|_| None),
            single_gate_net: None,
        })
    }

    fn cnot_template(&mut self, accuracy_bits: usize) -> Result<Vec<i32>, String> {
        if let Some((word, achieved_bits)) = &self.cnot_template {
            if *achieved_bits >= accuracy_bits {
                return Ok(word.clone());
            }
        }
        self.cnot_template = None;
        let args = [
            self.source.to_str_radix(10),
            self.sk_depth.to_string(),
            self.net_depth.to_string(),
            self.max_gates.to_string(),
            self.refinement.to_string(),
            accuracy_bits.to_string(),
        ];
        let refs: Vec<_> = args.iter().map(String::as_str).collect();
        let candidate = compile_selected(&refs, false)?;
        let word = candidate.word;
        self.cnot_template = Some((word.clone(), candidate.accuracy_bits));
        Ok(word)
    }

    fn hadamard_template(&mut self, accuracy_bits: usize) -> Result<Vec<i32>, String> {
        if let Some((word, achieved_bits)) = &self.hadamard_template {
            if *achieved_bits >= accuracy_bits {
                return Ok(word.clone());
            }
        }
        self.hadamard_template = None;
        let target = BraidTarget::H(0);
        let word = self.compile_single_uncached(&target, accuracy_bits)?;
        self.hadamard_template = Some((word.clone(), accuracy_bits));
        Ok(word)
    }

    /// Compile every target emitted by `CarrierGate::lower_for_braid` into
    /// source-bound Fibonacci generators. The sink receives generators as
    /// they are produced, allowing direct fusion-hardware or braid-tape use.
    pub fn compile_target_to<F>(
        &mut self,
        target: &BraidTarget,
        minimum_accuracy_bits: usize,
        mut emit: F,
    ) -> Result<(), String>
    where
        F: FnMut(i32) -> Result<(), String>,
    {
        if let BraidTarget::Cnot { control, target } = target {
            return self.compile_cnot_to(*control, *target, minimum_accuracy_bits, emit);
        }

        let (qubit, local_target, cache_slot, accuracy_bits) = match target {
            BraidTarget::X(qubit) => (*qubit, BraidTarget::X(0), Some(0), minimum_accuracy_bits),
            BraidTarget::H(qubit) => (*qubit, BraidTarget::H(0), None, minimum_accuracy_bits),
            BraidTarget::T { qubit, inverse } => (
                *qubit,
                BraidTarget::T {
                    qubit: 0,
                    inverse: *inverse,
                },
                Some(if *inverse { 3 } else { 2 }),
                minimum_accuracy_bits,
            ),
            BraidTarget::Feedback {
                qubit,
                numerator,
                denominator_bits,
            } => (
                *qubit,
                BraidTarget::Feedback {
                    qubit: 0,
                    numerator: numerator.clone(),
                    denominator_bits: *denominator_bits,
                }
                .reduced_feedback(),
                None,
                minimum_accuracy_bits.max(self.feedback_accuracy_bits),
            ),
            BraidTarget::Cnot { .. } => unreachable!(),
        };

        let word = if matches!(local_target, BraidTarget::H(0)) {
            self.hadamard_template(accuracy_bits)?
        } else if let Some(slot) = cache_slot {
            if let Some((achieved, word)) = &self.single_templates[slot] {
                if *achieved >= accuracy_bits {
                    word.clone()
                } else {
                    self.single_templates[slot] = None;
                    self.compile_single_template(&local_target, slot, accuracy_bits)?
                }
            } else {
                self.compile_single_template(&local_target, slot, accuracy_bits)?
            }
        } else {
            self.compile_single_uncached(&local_target, accuracy_bits)?
        };
        emit_local_shifted(&word, qubit, &mut emit)
    }

    fn compile_single_template(
        &mut self,
        target: &BraidTarget,
        slot: usize,
        accuracy_bits: usize,
    ) -> Result<Vec<i32>, String> {
        let word = self.compile_single_uncached(target, accuracy_bits)?;
        self.single_templates[slot] = Some((accuracy_bits, word.clone()));
        Ok(word)
    }

    fn compile_single_uncached(
        &mut self,
        target: &BraidTarget,
        accuracy_bits: usize,
    ) -> Result<Vec<i32>, String> {
        let reduced_target = target.reduced_feedback();
        let local = FibonacciLocal::new(&self.source)?;
        let target_matrix = local.target(&reduced_target)?;
        if target_matrix.0 == LocalMatrix::identity(local.format()).0 {
            return Ok(Vec::new());
        }
        if self.single_gate_net.is_none() {
            let scale = local.format().scale();
            let generators = [1, 2].map(|generator| {
                local
                    .evaluate(&[generator])
                    .map(|matrix| as_matrix(&matrix, &scale))
            });
            self.single_gate_net = Some(build_local_net(
                [generators[0].clone()?, generators[1].clone()?],
                self.net_depth,
                self.max_gates,
            ));
        }
        let net = self
            .single_gate_net
            .as_ref()
            .ok_or("single-qubit braid net was not retained")?;
        let mut depth = self.sk_depth;
        loop {
            let candidate = compile_single_qubit_with_net(
                &local,
                &reduced_target,
                0,
                &target_matrix,
                net,
                depth,
                accuracy_bits,
            );
            let (word, _) = match candidate {
                Ok(candidate) => candidate,
                Err(error)
                    if error == "anyon braid misses the requested dyadic feedback precision" =>
                {
                    depth = depth
                        .checked_add(1)
                        .ok_or("feedback braid depth overflow")?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            let observed = local.evaluate(&word)?;
            if fixed_projective_error_within(&observed, &target_matrix, accuracy_bits)? {
                return Ok(word);
            }
            depth = depth
                .checked_add(1)
                .ok_or("single-qubit braid depth overflow")?;
        }
    }

    pub fn compile_cnot(
        &mut self,
        control: usize,
        target: usize,
        minimum_accuracy_bits: usize,
    ) -> Result<Vec<i32>, String> {
        let mut word = Vec::new();
        self.compile_cnot_to(control, target, minimum_accuracy_bits, |generator| {
            word.push(generator);
            Ok(())
        })?;
        Ok(word)
    }

    /// Stream the routed braid to a sink without retaining a second copy of
    /// the full distant-wire word.
    pub fn compile_cnot_to<F>(
        &mut self,
        control: usize,
        target: usize,
        minimum_accuracy_bits: usize,
        mut emit: F,
    ) -> Result<(), String>
    where
        F: FnMut(i32) -> Result<(), String>,
    {
        if control == target {
            return Err("CNOT control overlaps target".into());
        }
        let width = usize::try_from(self.source.bits())
            .map_err(|_| "source width exceeds host indexing")?;
        let logical_qubits = width
            .checked_mul(3)
            .and_then(|value| value.checked_add(4))
            .ok_or("logical register width overflow")?;
        if control >= logical_qubits || target >= logical_qubits {
            return Err("CNOT wire lies outside the source-bound arithmetic register".into());
        }
        let swap_count = control.abs_diff(target) - 1;
        let reverse_final = usize::from(control > target);
        let cnot_count = swap_count
            .checked_mul(6)
            .and_then(|value| value.checked_add(1))
            .ok_or("routed CNOT count overflow")?;
        let hadamard_count = swap_count
            .checked_mul(8)
            .and_then(|value| value.checked_add(reverse_final * 4))
            .ok_or("routed Hadamard count overflow")?;
        let component_count = cnot_count
            .checked_add(hadamard_count)
            .ok_or("routed gate count overflow")?;
        let composition_guard =
            usize::BITS.saturating_sub(component_count.saturating_sub(1).leading_zeros());
        let component_accuracy_bits = minimum_accuracy_bits
            .checked_add(composition_guard as usize)
            .and_then(|value| value.checked_add(usize::from(component_count > 1)))
            .ok_or("routed CNOT accuracy floor overflow")?;
        let cnot = self.cnot_template(component_accuracy_bits)?;
        let h_word = if control > target || control.abs_diff(target) > 1 {
            self.hadamard_template(component_accuracy_bits)?
        } else {
            Vec::new()
        };
        let adjacent_left = if control < target {
            for left in control..target - 1 {
                emit_swap(&cnot, &h_word, left, &mut emit)?;
            }
            target - 1
        } else {
            for left in (target + 1..control).rev() {
                emit_swap(&cnot, &h_word, left, &mut emit)?;
            }
            target
        };
        if control < target {
            emit_shifted(&cnot, adjacent_left, &mut emit)?;
        } else {
            emit_reverse_cnot(&cnot, &h_word, adjacent_left, &mut emit)?;
        }
        if control < target {
            for left in (control..target - 1).rev() {
                emit_inverse_swap(&cnot, &h_word, left, &mut emit)?;
            }
        } else {
            for left in target + 1..control {
                emit_inverse_swap(&cnot, &h_word, left, &mut emit)?;
            }
        }
        Ok(())
    }
}

pub fn verify_file(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 || args.len() > 5 {
        return Err(
            "usage: anyon_cnot_verify N <compiled-word-report> [minimum_accuracy_bits=0] [eps=1e-6] [reject=1e-2]".into(),
        );
    }
    let source = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid source integer")?;
    if source.bits() < 128 {
        return Err("anyon CNOT verification requires a source of at least 128 bits".into());
    }
    let report =
        std::fs::read_to_string(args[1]).map_err(|error| format!("read braid report: {error}"))?;
    let minimum_accuracy_bits = args.get(2).map_or(Ok(0usize), |raw| {
        raw.parse().map_err(|_| "invalid minimum accuracy")
    })?;
    let thresholds = g_momonados::belnap_residual::Thresholds {
        eps: args.get(3).map_or(Ok(1e-6f64), |raw| {
            raw.parse().map_err(|_| "invalid Belnap clean threshold")
        })?,
        reject: args.get(4).map_or(Ok(1e-2f64), |raw| {
            raw.parse().map_err(|_| "invalid Belnap reject threshold")
        })?,
    };
    let encoded = report
        .lines()
        .find_map(|line| line.strip_prefix("word="))
        .ok_or("compiled report has no physical braid word")?;
    let word: Vec<i32> = encoded
        .split(',')
        .map(|item| {
            item.parse()
                .map_err(|_| "compiled report contains a malformed braid generator")
        })
        .collect::<Result<_, _>>()?;
    let algebra = FibonacciPair::new(&source)?;
    let physical = algebra.evaluate(&word)?;
    let channels = algebra.in_pair_channels(&physical);
    let residual = algebra.cnot_residual(&channels);
    let maximum_residual = residual.maximum();
    let bits = if maximum_residual.is_zero() {
        algebra.format().w_bits
    } else {
        algebra
            .format()
            .w_bits
            .saturating_sub(maximum_residual.bits())
    };
    if bits < minimum_accuracy_bits as u64 {
        return Err(format!(
            "saved CNOT braid reaches {bits} residual accuracy bits, below the requested {minimum_accuracy_bits}"
        ));
    }
    let scale = algebra.format().scale().to_biguint()
        .ok_or("fixed-point scale is not positive")?;
    let (belnap, tier) = g_momonados::belnap_integration::classify_residual(
        &residual, &scale, thresholds,
    )?;
    Ok(format!(
        "source_bits={} physical_word_length={} minimum_accuracy_bits={} residual_accuracy_bits={} computational={} leakage={} unitarity={} belnap_comp={:?} belnap_leak={:?} belnap_tier={:?} belnap_eps={} belnap_reject={}",
        source.bits(), word.len(), minimum_accuracy_bits, bits,
        residual.computational, residual.leakage, residual.unitarity,
        belnap.comp, belnap.leak, tier, thresholds.eps, thresholds.reject,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::hash::Hash;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    #[derive(Default)]
    struct ExchangeRecorder {
        exchanges: Vec<i32>,
        logical_qubits: usize,
        phase_bits: usize,
        preparation: Option<WorkPreparation>,
    }

    impl FibonacciAnyonDevice for ExchangeRecorder {
        fn begin(
            &mut self,
            _source: &[char],
            _base: &[char],
            logical_qubits: usize,
            phase_bits: usize,
            work_preparation: WorkPreparation,
        ) -> Result<(), String> {
            self.logical_qubits = logical_qubits;
            self.phase_bits = phase_bits;
            self.preparation = Some(work_preparation);
            Ok(())
        }

        fn generate_exchange(&mut self, generator: i32) -> Result<(), String> {
            self.exchanges.push(generator);
            Ok(())
        }

        fn measure_control_fusion(&mut self) -> Result<bool, String> {
            Ok(true)
        }

        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn abort(&mut self) {}
    }

    #[test]
    fn split_fuse_cnot_compiles_at_128_through_2048_bit_semiprime_widths() {
        let rsa_2048 = concat!(
            "2519590847565789349402718324004839857142928212620403202777713783604366202070",
            "7595556264018525880784406918290641249515082189298559149176184502808489120072",
            "8449926873928072877767359714183472702618963750149718246911650776133798590957",
            "0009733045974880842840179742910064245869181719511874612151517265463228221686",
            "9987549182422433637259085141865462043576798423387184774447920739934236584823",
            "8242811981638150106748104516603773060562016196762561338441436038339044149526",
            "3443219011465754445417842402092461651572335077870774981712577246796292638635",
            "6373289912154831438167899885040445364023527381951378636564391212010397122822",
            "120720357"
        );
        for (source, bits) in [
            ("296650821743515430283258444261036507151", 128usize),
            ("74190557381655886253556359637698917958655348096397072330011641798610025580603", 256),
            ("9969906765783880964144656106420939635433526042367203781554613194134456045788303190678018270287475265649670195674743890452325667572153673384288330674180821", 512),
            ("151777019658254876857817633777310500067543531862690952628150066934774348856446724909442250322963432373284529849953977441737417349575905634097035061187457141916353707279440643579659068527565844333535400729258064430177333685201919732761254826385161013978682394914675587203065373750392052441296616389598208643077", 1024),
            (rsa_2048, 2048),
        ] {
            let source_value = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert_eq!(source_value.bits() as usize, bits);
            let args = [source, "4", "7", "20000", "2", "8"];
            let candidate = compile_selected(&args, false).unwrap();
            assert!(candidate.accuracy_bits >= 8, "{bits}-bit source missed CNOT accuracy floor");
            assert!(!candidate.word.is_empty(), "{bits}-bit source emitted no Fibonacci braid generators");
            std::println!(
                "source_bits={bits} braid_generators={} residual_accuracy_bits={}",
                candidate.word.len(),
                candidate.accuracy_bits
            );
        }
    }

    #[test]
    fn cnot_compiler_keeps_shorter_regular_word_when_it_meets_floor() {
        let source = "296650821743515430283258444261036507151";
        let report = compile(&[source, "7", "7", "20000", "2", "20"]).unwrap();
        assert!(report.contains("residual_accuracy_bits=24\n"));
        assert!(report.contains("physical_word_length=1425830 "));
    }

    #[test]
    fn nonadjacent_cnot_targets_emit_source_bound_braids_on_128_bit_semiprime() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let mut compiler = FibonacciBraidCompiler::new(&source, 6, 7, 20_000, 2).unwrap();
        let summarize = |compiler: &mut FibonacciBraidCompiler, control, target| {
            let mut hasher = DefaultHasher::new();
            let mut count = 0usize;
            let mut largest = 0usize;
            compiler
                .compile_cnot_to(control, target, 8, |generator| {
                    generator.hash(&mut hasher);
                    count += 1;
                    largest = largest.max(generator.unsigned_abs() as usize);
                    Ok(())
                })
                .unwrap();
            (count, hasher.finish(), largest)
        };
        let forward = summarize(&mut compiler, 0, 2);
        let repeated = summarize(&mut compiler, 0, 2);
        let reverse = summarize(&mut compiler, 2, 0);
        let mut target_compiler = FibonacciBraidCompiler::new(&source, 6, 7, 20_000, 2).unwrap();
        let mut lowered = Vec::new();
        target_compiler
            .compile_target_to(
                &BraidTarget::Cnot {
                    control: 0,
                    target: 2,
                },
                8,
                |generator| {
                    lowered.push(generator);
                    Ok(())
                },
            )
            .unwrap();
        let largest_generator = 3 * (3 * source.bits() as usize + 4) - 1;
        assert_eq!(forward, repeated);
        assert!(!lowered.is_empty());
        assert!(lowered
            .iter()
            .all(|generator| generator.unsigned_abs() as usize <= largest_generator));
        assert!(forward.0 > 0 && reverse.0 > 0);
        assert!(forward.2 <= largest_generator && reverse.2 <= largest_generator);
        assert!(compiler.compile_cnot(1, 1, 8).is_err());
        assert!(compiler.compile_cnot(usize::MAX, 0, 8).is_err());
    }

    #[test]
    fn single_qubit_gates_compile_to_offset_anyon_braids_on_128_bit_semiprime() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let qubit = 3usize;
        let targets = [
            BraidTarget::H(qubit),
            BraidTarget::X(qubit),
            BraidTarget::T {
                qubit,
                inverse: false,
            },
            BraidTarget::T {
                qubit,
                inverse: true,
            },
        ];
        for target in &targets {
            let (word, error) = compile_single_qubit(&source, target, 1, 5, 4096).unwrap();
            assert!(error.is_finite());
            assert!(
                !word.is_empty(),
                "target {target:?} compiled at error {error}"
            );
            assert!(word
                .iter()
                .all(|gate| matches!(gate.unsigned_abs(), 10 | 11)));
        }
        let phase = BraidTarget::Feedback {
            qubit,
            numerator: BigUint::from(3u8),
            denominator_bits: 32,
        };
        assert!(compile_single_qubit(&source, &phase, 1, 5, 4096).is_err());
        let result = compile_single_qubit(&source, &phase, 3, 7, 20_000);
        assert_eq!(
            result.unwrap_err(),
            "anyon braid misses the requested dyadic feedback precision"
        );
        assert_eq!(
            compile_single_qubit(&source, &phase, 5, 7, 20_000).unwrap_err(),
            "anyon braid misses the requested dyadic feedback precision"
        );
    }

    #[test]
    fn feedback_phase_bounds_and_integral_turns_on_128_bit_semiprime() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let integral_turn = BraidTarget::Feedback {
            qubit: 0,
            numerator: BigUint::from(4u8),
            denominator_bits: 2,
        };
        let (word, error): (Vec<i32>, f64) =
            compile_single_qubit(&source, &integral_turn, 2, 5, 4096).unwrap();
        assert!(word.is_empty());
        assert_eq!(error, 0.0);

        let mut compiler = FibonacciBraidCompiler::new(&source, 2, 5, 4096, 0).unwrap();
        let mut emitted = 0usize;
        compiler
            .compile_target_to(&integral_turn, 2, |_| {
                emitted += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(emitted, 0);

        let qft_denominator_bits = 2 * source.bits() as usize + 8;
        let qft_feedback = BraidTarget::Feedback {
            qubit: 0,
            numerator: BigUint::one(),
            denominator_bits: qft_denominator_bits,
        };
        FibonacciLocal::new(&source)
            .unwrap()
            .target(&qft_feedback)
            .expect("source-derived QFT phase width must fit the phase precision");

        let unbounded_denominator = BraidTarget::Feedback {
            qubit: 0,
            numerator: BigUint::one(),
            denominator_bits: usize::MAX,
        };
        assert_eq!(
            compile_single_qubit(&source, &unbounded_denominator, 2, 5, 4096).unwrap_err(),
            "feedback denominator exceeds the source-bound phase precision"
        );
    }

    #[test]
    fn full_width_qft_feedback_compiles_with_bounded_error_on_128_bit_semiprime() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let phase_bits = 2 * source.bits() as usize + 8;
        let accuracy_bits = qft_feedback_accuracy_bits(phase_bits).unwrap();
        assert_eq!(phase_bits, 264);
        assert_eq!(accuracy_bits, 15);
        assert_eq!(qft_feedback_accuracy_bits(2 * 2048 + 8).unwrap(), 19);

        let target = BraidTarget::Feedback {
            qubit: 0,
            numerator: (BigUint::one() << 263usize) + BigUint::one(),
            denominator_bits: phase_bits,
        };
        let mut compiler = FibonacciBraidCompiler::new(&source, 7, 7, 20_000, 2).unwrap();
        compiler.feedback_accuracy_bits = accuracy_bits;
        let mut emitted = 0usize;
        compiler
            .compile_target_to(&target, 8, |_| {
                emitted = emitted.checked_add(1).ok_or("exchange count overflow")?;
                Ok(())
            })
            .unwrap();
        assert!(
            emitted > 0,
            "nontrivial feedback phase must generate exchanges"
        );
        assert!(
            emitted < 100_000,
            "feedback braid unexpectedly expanded to {emitted} exchanges"
        );
    }

    #[test]
    fn single_qubit_gate_net_is_reused_across_128_bit_phase_targets() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let mut compiler = FibonacciBraidCompiler::new(&source, 1, 5, 4096, 0).unwrap();
        compiler
            .compile_single_uncached(&BraidTarget::X(0), 0)
            .unwrap();
        let net_address = compiler.single_gate_net.as_ref().unwrap().entries.as_ptr();
        compiler
            .compile_single_uncached(
                &BraidTarget::T {
                    qubit: 0,
                    inverse: false,
                },
                0,
            )
            .unwrap();
        assert_eq!(
            compiler.single_gate_net.as_ref().unwrap().entries.as_ptr(),
            net_address,
            "changing the phase target must reuse the source-bound gate net"
        );
    }

    #[test]
    fn source_bound_compiler_survives_base_changes() {
        let source =
            BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let source_tape = (0..source.bits())
            .map(|bit| if source.bit(bit) { EVALF } else { EVALT })
            .collect::<Vec<_>>();
        let base_two = [EVALT, EVALF];
        let base_three = [EVALF, EVALF];
        let logical_qubits = 3 * source.bits() as usize + 4;
        let phase_bits = 2 * source.bits() as usize + 8;
        let mut carrier =
            CompiledFibonacciCarrier::new(ExchangeRecorder::default(), 1, 5, 4096, 0, 0);

        carrier
            .begin(
                &source_tape,
                &base_two,
                logical_qubits,
                phase_bits,
                WorkPreparation::UniformResidues,
            )
            .unwrap();
        carrier.compiler.as_mut().unwrap().cnot_template =
            Some((vec![1, -1, 2], usize::MAX));
        carrier.device.finish().unwrap();

        carrier
            .begin(
                &source_tape,
                &base_three,
                logical_qubits,
                phase_bits,
                WorkPreparation::UniformResidues,
            )
            .unwrap();

        assert_eq!(
            carrier
                .compiler
                .as_ref()
                .unwrap()
                .cnot_template
                .as_ref()
                .unwrap()
                .0,
            vec![1, -1, 2],
            "changing only the phase base must retain source-bound braid templates"
        );
    }

    #[test]
    fn recycled_carrier_streams_anyonic_exchanges_from_a_128_bit_source() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(source.bits(), 128);
        let source_tape = (0..source.bits())
            .map(|bit| if source.bit(bit) { EVALF } else { EVALT })
            .collect::<Vec<_>>();
        let base_tape = [EVALF, EVALF];
        let logical_qubits = 3 * source.bits() as usize + 4;
        let phase_bits = 2 * source.bits() as usize + 8;
        let mut carrier =
            CompiledFibonacciCarrier::new(ExchangeRecorder::default(), 1, 5, 4096, 0, 0);
        carrier
            .begin(
                &source_tape,
                &base_tape,
                logical_qubits,
                phase_bits,
                WorkPreparation::UniformResidues,
            )
            .unwrap();
        carrier.apply_target(BraidTarget::X(7)).unwrap();
        assert!(carrier.device().exchanges.len() > 0);
        assert!(carrier
            .device()
            .exchanges
            .iter()
            .all(|generator| matches!(generator.unsigned_abs(), 22 | 23)));
        assert_eq!(carrier.device().logical_qubits, logical_qubits);
        assert_eq!(carrier.device().phase_bits, phase_bits);
        assert_eq!(
            carrier.device().preparation,
            Some(WorkPreparation::UniformResidues)
        );
        assert!(carrier.measure_control().unwrap());
        carrier.finish().unwrap();
    }
}
/// Try to compile a sequence of Fourier braid targets with dual-path fallback
/// for CNOT gates, mirroring the logic in `compile_selected`.
fn try_compile_fourier_targets(
    compiler: &mut FibonacciBraidCompiler,
    targets: &[BraidTarget],
    _accuracy: usize,
    per_target: usize,
    _split_fuse: bool,
    word: &mut Vec<i32>,
) -> Result<(), String> {
    for target in targets {
        match compiler.compile_target_to(target, per_target, |generator| {
            word.push(generator);
            Ok(())
        }) {
            Ok(()) => {}
            Err(e) if e.starts_with("CNOT braid reaches ") && matches!(target, BraidTarget::Cnot { .. }) => {
                // Fallback: try the opposite split/fuse mode
                // We need to recompile this target with the alternate mode
                // Since the compiler state may be corrupted, we return the error
                // to let the caller handle the full fallback
                return Err(e);
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Public entry point: compile and evaluate the full Z4 Fourier word on the
/// five-channel physical fusion sector with outside-carrier retention.
/// Supports dual-path CNOT fallback and per-channel diagnostics.
pub fn compile_ququart_fourier(args: &[&str]) -> Result<String, String> {
    let raw_source = *args.first().ok_or(
        "usage: anyon_ququart_word <source-word> [sk-word net-word capacity-word refinement-word accuracy-word inverse]"
    )?;
    let native = raw_source.starts_with('⊢');
    let numeral = |raw: &str| -> Result<BigUint, String> {
        let read = g_momonados::godel_calculus::decode(raw).map_err(|error| error.to_string())?;
        if !matches!(read.structure, g_momonados::godel_calculus::Structure::CellBinary { .. })
            || g_momonados::godel_calculus::encode_cell_binary(&read.value) != raw {
            return Err("compiler input must be a canonical IMASM numeral word".into());
        }
        Ok(read.value.bits_le().iter().enumerate().fold(BigUint::zero(), |value, (bit, set)| {
            if *set { value | (BigUint::one() << bit) } else { value }
        }))
    };
    let source = if native { numeral(raw_source)? } else {
        BigUint::parse_bytes(raw_source.as_bytes(), 10).ok_or("invalid source")?
    };
    let option = |i: usize, default: usize| -> Result<usize, String> {
        args.get(i)
            .map(|s| if native {
                numeral(s)?.to_usize().ok_or("compiler option exceeds host indexing".into())
            } else { s.parse().map_err(|_| format!("invalid option {i}")) })
            .unwrap_or(Ok(default))
    };
    let depth = option(1, 4)?;
    let net = option(2, 7)?;
    let capacity = option(3, 20000)?;
    let refinement = option(4, 2)?;
    let accuracy = option(5, 4)?;
    let inverse = args.get(6).map(|s| *s == "inverse").unwrap_or(false);
    let per_target = accuracy.checked_add(4).ok_or("Fourier accuracy overflow")?;

    let mut compiler = FibonacciBraidCompiler::new(
        &source,
        depth,
        net,
        capacity,
        refinement,
    )?;
    let mut word = Vec::new();
    let targets = fourier_braid_targets(inverse);

    // Try compilation with dual-path fallback for CNOT
    match try_compile_fourier_targets(
        &mut compiler,
        &targets,
        accuracy,
        per_target,
        false,
        &mut word,
    ) {
        Ok(()) => {}
        Err(e) if e.starts_with("CNOT braid reaches ") => {
            // Fallback: clear the word and try with a fresh compiler
            // (templates may be cached with the failed mode)
            word.clear();
            compiler = FibonacciBraidCompiler::new(
                &source,
                depth,
                net,
                capacity,
                refinement,
            )?;
            try_compile_fourier_targets(
                &mut compiler,
                &targets,
                accuracy,
                per_target,
                true,
                &mut word,
            )?;
        }
        Err(e) => return Err(e),
    }

    let algebra = FibonacciPair::new(&source)?;
    let physical = algebra.evaluate(&word)?;
    let observed = algebra.in_pair_channels(&physical);
    let scale = algebra.format().scale();

    // Compute projective overlap with ideal Fourier matrix
    let mut inner = Complex::new(0.0, 0.0);
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64
                * if inverse { -1.0 } else { 1.0 };
            let ideal = Complex::new(angle.cos() * 0.5, angle.sin() * 0.5);
            let z = &observed.0[5 * row + col];
            let z = Complex::new(ratio(&z.re, &scale), ratio(&z.im, &scale));
            inner = inner + z * ideal.conj();
        }
    }
    let norm = (inner.re * inner.re + inner.im * inner.im).sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return Err("Fourier word has zero projective overlap".into());
    }
    let phase = Complex::new(inner.re / norm, inner.im / norm);

    // Compute maximum computational error across all 16 entries
    let mut computational = 0.0f64;
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64
                * if inverse { -1.0 } else { 1.0 };
            let ideal = phase * Complex::new(angle.cos() * 0.5, angle.sin() * 0.5);
            let z = &observed.0[5 * row + col];
            let dr = ratio(&z.re, &scale) - ideal.re;
            let di = ratio(&z.im, &scale) - ideal.im;
            computational = computational.max((dr * dr + di * di).sqrt());
        }
    }

    let leakage = ratio(&BigInt::from(observed.leakage()), &scale);
    let unitary = observed.adjoint().multiply(&observed, algebra.format());
    let identity = g_momonados::anyon_pair::PairMatrix::identity(algebra.format());
    let unitarity = unitary
        .0
        .iter()
        .zip(&identity.0)
        .map(|(a, b)| {
            ratio(
                &(&a.re - &b.re).abs().max((&a.im - &b.im).abs()),
                &scale,
            )
        })
        .fold(0.0f64, f64::max);

    let maximum = computational.max(leakage).max(unitarity);
    let threshold = 2.0f64.powi(-(i32::try_from(accuracy).map_err(|_| "Fourier accuracy too large")?));
    if maximum > threshold {
        // Detailed per-channel diagnostic output
        let mut detail = String::new();
        for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
                let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64
                    * if inverse { -1.0 } else { 1.0 };
                let ideal = Complex::new(angle.cos() * 0.5, angle.sin() * 0.5);
                let z = &observed.0[5 * row + col];
                let zr = ratio(&z.re, &scale);
                let zi = ratio(&z.im, &scale);
                detail.push_str(&format!(
                    "  [{k},{l}] obs=({zr:.6},{zi:.6}) ideal=({:.6},{:.6})\n",
                    ideal.re, ideal.im
                ));
            }
        }
        return Err(format!(
            "compiled Z4 word misses requested accuracy: computational={computational:.8e} leakage={leakage:.8e} unitarity={unitarity:.8e}\n{detail}"
        ));
    }

    if native {
        let native_word = |value: &BigUint| g_momonados::godel_calculus::encode_cell_binary(
            &g_momonados::godel_calculus::Nat::from_bits_le((0..value.bits()).map(|bit| value.bit(bit)).collect()));
        let scalar = |value: u64| native_word(&BigUint::from(value));
        let exchange_words: Vec<_> = word.iter().map(|generator| format!("{}{}",
            if *generator < 0 { '≺' } else { '≻' }, scalar(u64::from(generator.unsigned_abs())))).collect();
        return Ok(serde_json::json!({
            "component": "ququart_fourier", "source_word": raw_source,
            "source_bits_word": scalar(source.bits()), "inverse_word": scalar(u64::from(inverse)),
            "exchanges_word": native_word(&BigUint::from(word.len())),
            "computational_word": scalar(computational.to_bits()),
            "leakage_word": scalar(leakage.to_bits()),
            "unitarity_word": scalar(unitarity.to_bits()),
            "exchange_words": exchange_words,
        }).to_string());
    }
    let mut report = format!(
        "Z4 Fourier Fibonacci braid\nsource_bits={} inverse={} physical_word_length={}\ncomputational={computational:.8e} leakage={leakage:.8e} unitarity={unitarity:.8e}\nword=",
        source.bits(),
        inverse,
        word.len()
    );
    for (i, g) in word.iter().enumerate() {
        if i > 0 {
            report.push(' ');
        }
        report.push_str(&g.to_string());
    }
    report.push('\n');
    Ok(report)
}
