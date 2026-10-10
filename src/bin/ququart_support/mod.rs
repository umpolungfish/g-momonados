use g_momonados::anyon_pair::{FibonacciPair, PairMatrix, COMPUTATIONAL_CHANNELS};
use g_momonados::anyon_ququart::{QuquartCarrier, QuquartDigit};
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, ToPrimitive};
#[allow(dead_code)]
pub mod work;

pub struct Metrics { pub computational: f64, pub leakage: f64, pub closure: f64, pub exchanges: usize }

pub fn validate_base_scaling(prepared: &serde_json::Value) -> Result<(), String> {
    if prepared.get("binary_base_word").is_none() { return Ok(()); }
    let read = |field: &str| -> Result<BigUint,String> {
        numeral(prepared[field].as_str().ok_or_else(|| format!("missing {field}"))?)
    };
    let radix = prepared["radix_word"].as_str().ok_or("missing scaling radix word")?;
    let expected = g_momonados::ququart_factor::scaled_modular_base(
        &read("source_word")?, &read("binary_base_word")?, radix)?;
    if read("base_word")? != expected {
        return Err("modular base differs from binary base scaled to the work radix".into());
    }
    Ok(())
}

pub fn numeral(word: &str) -> Result<BigUint, String> {
    g_momonados::godel_calculus::decode_cell_biguint(word)
}
pub fn signed_numeral(value: &str) -> Result<BigInt, String> {
    let mut chars = value.chars();
    let sign = match chars.next() {
        Some('≺') => num_bigint::Sign::Minus,
        Some('≻') => num_bigint::Sign::Plus,
        _ => return Err("signed coordinate must begin with an IMASM direction".into()),
    };
    Ok(BigInt::from_biguint(sign, numeral(chars.as_str())?))
}
fn ratio(n: &BigInt, d: &BigInt) -> f64 {
    let shift = n.bits().max(d.bits()).saturating_sub(900) as usize;
    (n >> shift).to_f64().unwrap_or(0.0) / (d >> shift).to_f64().unwrap_or(1.0)
}

// Recompute the action of the exact coordinates entering resident execution.
// Saved diagnostics describe preparation; they cannot authorize a changed map.
fn matrix_metrics(observed: &PairMatrix, format: &g_momonados::phase_unbraid::FixedPointFormat,
                  exchanges: usize) -> Result<Metrics, String> {
    let scale = format.scale();
    let mut overlap = (0.0f64, 0.0f64);
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64;
            let z = &observed.0[5 * row + col];
            let (re, im) = (ratio(&z.re, &scale), ratio(&z.im, &scale));
            overlap.0 += (re * angle.cos() + im * angle.sin()) * 0.5;
            overlap.1 += (im * angle.cos() - re * angle.sin()) * 0.5;
        }
    }
    let norm = overlap.0.hypot(overlap.1);
    if !norm.is_finite() || norm == 0.0 { return Err("zero Fourier overlap".into()); }
    let phase = (overlap.0 / norm, overlap.1 / norm);
    let mut computational = 0.0f64;
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64;
            let ideal = ((phase.0 * angle.cos() - phase.1 * angle.sin()) * 0.5,
                         (phase.0 * angle.sin() + phase.1 * angle.cos()) * 0.5);
            let z = &observed.0[5 * row + col];
            computational = computational.max((ratio(&z.re, &scale) - ideal.0)
                .hypot(ratio(&z.im, &scale) - ideal.1));
        }
    }
    let leakage = ratio(&BigInt::from(observed.leakage()), &scale);
    let adjoint = observed.adjoint();
    let closure = identity_residual(&adjoint.multiply(observed, format), format)
        .max(identity_residual(&observed.multiply(&adjoint, format), format));
    Ok(Metrics { computational, leakage, closure, exchanges })
}

fn identity_residual(returned: &PairMatrix, format: &g_momonados::phase_unbraid::FixedPointFormat) -> f64 {
    let identity = PairMatrix::identity(format);
    let scale = format.scale();
    returned.0.iter().zip(identity.0.iter()).map(|(a, b)| {
        ratio(&(&a.re - &b.re).abs().max((&a.im - &b.im).abs()), &scale)
    }).fold(0.0f64, f64::max)
}

fn check_accuracy(metrics: &Metrics, exponent: i32) -> Result<(), String> {
    if !metrics.computational.is_finite() || !metrics.leakage.is_finite() || !metrics.closure.is_finite()
        || metrics.computational.max(metrics.leakage).max(metrics.closure) > 2.0f64.powi(-exponent) {
        return Err(format!("Fourier membrane residual: computational={:.8e} leakage={:.8e} return={:.8e}",
            metrics.computational, metrics.leakage, metrics.closure));
    }
    Ok(())
}

fn native_fourier(source: &BigUint, exponent: i32) -> Result<(PairMatrix, Metrics), String> {
    let algebra = FibonacciPair::new(source)?;
    let format = algebra.format();
    let mut matrix = PairMatrix::identity(format);
    for digit in 0..4 {
        let mut column = QuquartCarrier::basis(&algebra, QuquartDigit::try_from(digit as u8)?);
        column.fourier_target(false);
        for row in 0..5 {
            matrix.0[5 * row + COMPUTATIONAL_CHANNELS[digit]] = column.amplitudes()[row].clone();
        }
    }
    let metrics = matrix_metrics(&matrix, format, 0)?;
    check_accuracy(&metrics, exponent)?;
    Ok((matrix, metrics))
}

pub fn contract(prepared: &serde_json::Value) -> Result<(BigUint, PairMatrix, Metrics), String> {
    let raw = prepared["source_word"].as_str().ok_or("missing source word")?;
    let n = numeral(raw)?;
    let accuracy = numeral(prepared["accuracy_word"].as_str().ok_or("missing accuracy word")?)?
        .to_u64().ok_or("invalid accuracy word")?;
    let exponent = i32::try_from(accuracy).map_err(|_| "accuracy overflow")?;
    let native = match prepared.get("operator_kind") {
        None => false,
        Some(value) if value.as_str() == Some("native_fourier") => true,
        Some(_) => return Err("unknown Fourier operator kind".into()),
    };
    if native && prepared.get("prepared_operator").is_none() {
        let (matrix, metrics) = native_fourier(&n, exponent)?;
        return Ok((n, matrix, metrics));
    }
    if let Some(operator) = prepared.get("prepared_operator") {
        let format = g_momonados::phase_unbraid::FixedPointFormat::for_modulus(&n)?;
        if numeral(operator["w_bits_word"].as_str().ok_or("missing precision word")?)?.to_u64() != Some(format.w_bits) {
            return Err("prepared operator precision differs from its source".into());
        }
        let entries = operator["matrix"].as_array().ok_or("missing prepared operator entries")?;
        let mut values = Vec::new();
        for entry in entries {
            let coordinate = |field: &str| -> Result<BigInt, String> {
                signed_numeral(entry[field].as_str().ok_or_else(|| format!("missing operator {field}"))?)
            };
            values.push(g_momonados::phase_unbraid::FixedComplex {
                re: coordinate("re_word")?, im: coordinate("im_word")?,
            });
        }
        let matrix = PairMatrix(values.try_into().map_err(|_| "prepared operator must contain all five fusion channels")?);
        let integer = |field: &str| -> Result<BigUint, String> {
            numeral(operator[field].as_str().ok_or_else(|| format!("missing prepared {field}"))?)
        };
        let metric = |field: &str| -> Result<f64, String> {
            let value = f64::from_bits(integer(field)?.to_u64().ok_or("invalid diagnostic word")?);
            if value.is_finite() && value >= 0.0 { Ok(value) }
            else { Err(format!("invalid prepared {field}")) }
        };
        // Retain the physical inverse-word diagnostic, and additionally check
        // the loaded matrix in both directions at this representation boundary.
        metric("computational_word")?;
        metric("leakage_word")?;
        let physical_closure = metric("closure_word")?;
        let exchanges = integer("exchanges_word")?.to_usize()
            .ok_or("invalid prepared exchange count")?;
        if native {
            if exchanges != 0 { return Err("native Fourier operator claims braid exchanges".into()); }
            let (expected, _) = native_fourier(&n, exponent)?;
            if matrix.0.iter().zip(expected.0.iter()).any(|(observed, bound)|
                observed.re != bound.re || observed.im != bound.im) {
                return Err("baked native Fourier matrix differs from its source".into());
            }
        } else if exchanges == 0 {
            return Err("prepared Fourier operator has no exchanges".into());
        }
        let mut metrics = matrix_metrics(&matrix, &format, exchanges)?;
        metrics.closure = metrics.closure.max(physical_closure);
        check_accuracy(&metrics, exponent)?;
        return Ok((n, matrix, metrics));
    }
    let word: Vec<i32> = if let Some(exchanges) = prepared["exchange_words"].as_array() {
        exchanges.iter().map(|value| {
            let generator = signed_numeral(value.as_str().ok_or("exchange must be an IMASM word")?)?
                .to_i32().ok_or("invalid exchange word")?;
            if !(1..=5).contains(&generator.unsigned_abs()) {
                return Err("exchange word is outside the six-strand braid".into());
            }
            Ok(generator)
        }).collect::<Result<_, String>>()?
    } else {
        return Err("Fourier preparation requires IMASM exchange words".into());
    };
    if word.is_empty() { return Err("Fourier preparation has no exchange words".into()); }
    let pair = FibonacciPair::new(&n)?;
    let physical = pair.evaluate(&word)?;
    let observed = pair.in_pair_channels(&physical);
    let inverse: Vec<_> = word.iter().rev().map(|g| -*g).collect();
    let returned = pair.evaluate(&inverse)?.multiply(&physical, pair.format());
    let mut metrics = matrix_metrics(&observed, pair.format(), word.len())?;
    metrics.closure = metrics.closure.max(identity_residual(&returned, pair.format()));
    check_accuracy(&metrics, exponent)?;
    Ok((n, observed, metrics))
}
