//! Compile the derived local corrections around the controlled Fibonacci
//! exchange, then validate the complete emitted six-strand word.
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use g_momonados::anyon_local::{cnot_local_corrections, FibonacciLocal, LocalMatrix};
use g_momonados::anyon_pair::{FibonacciPair, COMPUTATIONAL_CHANNELS};
use g_momonados::phase_unbraid::{FixedComplex, FixedPointFormat};
use g_momonados::recycled_carrier::BraidTarget;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::fibonacci_qc::{sk_split_fuse, solovay_kitaev, Complex, GateNet, Matrix2};

fn ratio(value: &BigInt, scale: &BigInt) -> f64 {
    let shift = value.bits().max(scale.bits()).saturating_sub(900) as usize;
    (value >> shift).to_f64().unwrap_or(0.0)
        / (scale >> shift).to_f64().unwrap_or(1.0)
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
                        return Err(format!("sigma_{generator} leaks from the addressed logical triple"));
                    }
                }
            }
        }
        let block = |channels: [usize; 2]| Matrix2::from_array(core::array::from_fn(|i| {
            let cell = &recoupled.0[5 * channels[i / 2] + channels[i % 2]];
            Complex::new(ratio(&cell.re, &scale), ratio(&cell.im, &scale))
        }));
        let first = block(bit_channels[0]);
        let second = block(bit_channels[1]);
        let spectator_error = first.data.iter().zip(second.data.iter()).fold(0.0f64, |bound, (a, b)| {
            bound.max((a.re - b.re).abs()).max((a.im - b.im).abs())
        });
        if spectator_error > 1e-6 {
            return Err(format!("sigma_{generator} changes phase across spectator sectors: {spectator_error:.8e}"));
        }
        Ok([first, second])
    };
    let physical_generators = if target { [4usize, 5usize] } else { [1usize, 2usize] };
    let first = blocks(physical_generators[0])?;
    let second = blocks(physical_generators[1])?;
    Ok([first[0], second[0]])
}

fn build_local_net(generators: [Matrix2; 2], depth: usize, limit: usize) -> GateNet {
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
                if last == Some(-symbol) { continue; }
                if entries.len() >= limit { return GateNet { entries, reached_cap: true }; }
                let mut word = entries[index].0.clone();
                word.push(symbol);
                entries.push((word, gate.mul(parent)));
            }
        }
        if entries.len() == hi { break; }
        lo = hi;
        hi = entries.len();
    }
    GateNet { entries, reached_cap: false }
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
    if max_gates == 0 {
        return Err("net capacity must be positive".into());
    }
    let qubit = match target {
        BraidTarget::X(qubit)
        | BraidTarget::H(qubit)
        | BraidTarget::T { qubit, .. }
        | BraidTarget::Feedback { qubit, .. } => *qubit,
        BraidTarget::Cnot { .. } => return Err("single-qubit compiler received CNOT".into()),
    };
    let algebra = FibonacciLocal::new(source)?;
    let scale = algebra.format().scale();
    let local_target = algebra.target(target)?;
    let generators = [1, 2].map(|generator| {
        algebra
            .evaluate(&[generator])
            .map(|matrix| as_matrix(&matrix, &scale))
    });
    let generators = [generators[0].clone()?, generators[1].clone()?];
    let net = build_local_net(generators, net_depth, max_gates);
    if net.entries.is_empty() {
        return Err("single-qubit braid net is empty".into());
    }
    let (word, error) = compile_local_split(&local_target, &scale, &net, sk_depth)?;
    if word.is_empty() {
        return Err("braid synthesis collapsed a nontrivial gate to the identity".into());
    }
    let target_float = as_matrix(&local_target, &scale);
    let identity_error = Matrix2::projective_distance(&target_float, &Matrix2::identity());
    if error >= identity_error {
        return Err("braid synthesis did not improve on the identity approximation".into());
    }
    if let BraidTarget::Feedback {
        denominator_bits, ..
    } = target
    {
        let width = usize::try_from(source.bits()).map_err(|_| "source width exceeds host indexing")?;
        let precision_limit = width
            .checked_mul(2)
            .and_then(|value| value.checked_add(8))
            .ok_or("feedback precision limit overflow")?;
        if *denominator_bits > precision_limit {
            return Err("feedback denominator exceeds the source-bound phase precision".into());
        }
        let observed = algebra.evaluate(&word)?;
        if !fixed_projective_error_within(
            &observed,
            &local_target,
            *denominator_bits,
        )? {
            return Err("anyon braid misses the requested dyadic feedback precision".into());
        }
    }
    let first = qubit
        .checked_mul(3)
        .and_then(|offset| offset.checked_add(1))
        .ok_or("logical qubit strand offset overflow")?;
    let second = first.checked_add(1).ok_or("logical qubit strand offset overflow")?;
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
    let zero = FixedComplex { re: BigInt::from(0), im: BigInt::from(0) };
    Ok(LocalMatrix([lower, zero.clone(), zero, upper]))
}

pub fn compile(args: &[&str]) -> Result<String, String> {
    if args.is_empty() || args.len() > 6 {
        return Err("usage: anyon_cnot_word N [sk_depth=2] [net_depth=7] [max_gates=20000] [exchange_refinement=2] [minimum_accuracy_bits=0]".into());
    }
    let source = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid source integer")?;
    if source.bits() < 128 {
        return Err("anyon CNOT compilation requires a source of at least 128 bits".into());
    }
    let parse = |index: usize, default: usize, name: &str| -> Result<usize, String> {
        args.get(index).map_or(Ok(default), |raw| raw.parse().map_err(|_| format!("invalid {name}")))
    };
    let sk_depth = parse(1, 2, "SK depth")?;
    let net_depth = parse(2, 7, "net depth")?;
    let max_gates = parse(3, 20_000, "net capacity")?;
    let refinement = parse(4, 2, "exchange refinement")?;
    let minimum_accuracy_bits = parse(5, 0, "minimum accuracy")?;
    if max_gates == 0 { return Err("net capacity must be positive".into()); }

    let pair = FibonacciPair::new(&source)?;
    let local = FibonacciLocal::new(&source)?;
    let corrections = cnot_local_corrections(&source)?;
    let phase = local_phase_target(local.format())?;
    let target_net = build_local_net(pair_local_generators(&pair, true)?, net_depth, max_gates);
    let control_net = build_local_net(pair_local_generators(&pair, false)?, net_depth, max_gates);
    let scale = local.format().scale();
    let (basis, basis_error) = compile_local(&corrections.target_basis, &scale, &target_net, sk_depth)?;
    let (target_y, y_error) = compile_local(&corrections.target_y, &scale, &target_net, sk_depth)?;
    let (control_phase, phase_error) = compile_local(&phase, &scale, &control_net, sk_depth)?;

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
    let residual_bits = if residual.maximum().is_zero() { "exact".to_string() }
        else { format!("2^-{}", (pair.format().w_bits as usize).saturating_sub(residual.maximum().bits() as usize)) };
    let mut output = format!(
        "source_bits={} sk_depth={} net_depth={} net_capacity={} refinement={} minimum_accuracy_bits={} residual_accuracy_bits={}\nlocal_projective_errors basis={:.8e} target_y={:.8e} control_phase={:.8e}\nphysical_word_length={} residual={residual_bits}\ncomputational={} leakage={} unitarity={}\nword=",
        source.bits(), sk_depth, net_depth, max_gates, refinement,
        minimum_accuracy_bits, residual_accuracy_bits,
        basis_error, y_error, phase_error, word.len(), residual.computational,
        residual.leakage, residual.unitarity,
    );
    for (index, gate) in word.iter().enumerate() {
        if index != 0 { output.push(','); }
        output.push_str(&gate.to_string());
    }
    Ok(output)
}

pub fn verify_file(args: &[&str]) -> Result<String, String> {
    if args.len() != 2 {
        return Err("usage: anyon_cnot_verify N <compiled-word-report>".into());
    }
    let source = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid source integer")?;
    if source.bits() < 128 {
        return Err("anyon CNOT verification requires a source of at least 128 bits".into());
    }
    let report = std::fs::read_to_string(args[1]).map_err(|error| format!("read braid report: {error}"))?;
    let encoded = report.lines().find_map(|line| line.strip_prefix("word="))
        .ok_or("compiled report has no physical braid word")?;
    let word: Vec<i32> = encoded.split(',').map(|item| {
        item.parse().map_err(|_| "compiled report contains a malformed braid generator")
    }).collect::<Result<_, _>>()?;
    let algebra = FibonacciPair::new(&source)?;
    let physical = algebra.evaluate(&word)?;
    let channels = algebra.in_pair_channels(&physical);
    let residual = algebra.cnot_residual(&channels);
    let scale = algebra.format().scale().to_biguint().ok_or("invalid CNOT scale")?;
    let bits = if residual.maximum().is_zero() { 0 }
        else { scale.bits().saturating_sub(residual.maximum().bits()) };
    Ok(format!(
        "source_bits={} physical_word_length={} residual_accuracy_bits={} computational={} leakage={} unitarity={}",
        source.bits(), word.len(), bits, residual.computational, residual.leakage, residual.unitarity,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_qubit_gates_compile_to_offset_anyon_braids_on_128_bit_semiprime() {
        let source = BigUint::parse_bytes(
            b"296650821743515430283258444261036507151",
            10,
        )
        .unwrap();
        assert_eq!(source.bits(), 128);
        let qubit = 3usize;
        let targets = [
            BraidTarget::H(qubit),
            BraidTarget::X(qubit),
            BraidTarget::T { qubit, inverse: false },
            BraidTarget::T { qubit, inverse: true },
        ];
        for target in &targets {
            let (word, error) = compile_single_qubit(&source, target, 1, 5, 4096).unwrap();
            assert!(error.is_finite());
            assert!(!word.is_empty(), "target {target:?} compiled at error {error}");
            assert!(word.iter().all(|gate| matches!(gate.unsigned_abs(), 10 | 11)));
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
}
