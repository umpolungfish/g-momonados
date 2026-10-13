//! Apply a certified dimension-two split/fuse operator to one N-only resident state.
use g_momonados::{resident_glut::ResidentGlut, word_tape::WordTape};
use num_traits::{Signed, Zero};
use std::{env, fs, time::Instant};

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let source_path = args.next().ok_or("usage: resident_circuit_braid_probe N_ONLY_IMASM OPERATOR_JSON")?;
    let receipt_path = args.next().ok_or("missing split/fuse operator receipt")?;
    if args.next().is_some() { return Err("expected exactly two input paths".into()); }
    let source_text = fs::read_to_string(&source_path).map_err(|e| e.to_string())?;
    let source = WordTape::from_canonical_word(source_text.trim()).ok_or("invalid N-only IMASM source")?;
    let receipt_text = fs::read_to_string(&receipt_path).map_err(|e| e.to_string())?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_text).map_err(|e| e.to_string())?;
    let entries = receipt["synthesized_matrix"].as_array().ok_or("operator matrix is missing")?;
    if entries.len() != 4 || !receipt["unitary"].as_bool().unwrap_or(false)
        || receipt["word_matrix_residual"].as_f64().unwrap_or(f64::INFINITY) >= 1e-6 {
        return Err("operator receipt lacks a passing split/fuse word check".into());
    }
    let matrix = core::array::from_fn(|i| (entries[i]["re"].as_f64().unwrap(), entries[i]["im"].as_f64().unwrap()));
    let started = Instant::now();
    let mut resident = ResidentGlut::from_word(source.as_word())?;
    let operator = resident.apply_circuit_braid(&matrix)?;
    // Exercise the reverse arrow on this same retained diagram. The Q52
    // matrix is unitary up to coefficient rounding; the second application
    // contributes another 2^52 input-amplitude scale.
    let inverse = [(matrix[0].0, -matrix[0].1), (matrix[2].0, -matrix[2].1),
        (matrix[1].0, -matrix[1].1), (matrix[3].0, -matrix[3].1)];
    let inverse_operator = resident.apply_circuit_braid(&inverse)?;
    // gram_before was recorded after the first 2^52 amplitude rescale.
    // The inverse call contributes one further 2^52 amplitude rescale, so
    // its Gram coordinates scale by 2^104 relative to that snapshot.
    let recovery_scale = num_bigint::BigInt::from(1u8) << 104usize;
    let recovery_deltas = operator.gram_before.iter().zip(&inverse_operator.gram_after)
        .map(|((before_re,before_im),(after_re,after_im))|
            (after_re - before_re * &recovery_scale, after_im - before_im * &recovery_scale))
        .collect::<Vec<_>>();
    let exact_recovered_coordinates = recovery_deltas.iter().filter(|(re,im)| re.is_zero() && im.is_zero()).count();
    let maximum_recovery_delta = recovery_deltas.iter().map(|(re,im)| re.abs().max(im.abs()))
        .max().unwrap_or_default();
    let sic = resident.sic_witness()?;
    let complex_words = |values: &[(num_bigint::BigInt, num_bigint::BigInt)]| values.iter().map(|(re, im)|
        serde_json::json!({"re":re.to_string(),"im":im.to_string()})).collect::<Vec<_>>();
    let report = serde_json::json!({
        "source_path":source_path,
        "source_bits":source.bit_len(),
        "operator_receipt_path":receipt_path,
        "circuit":receipt["circuit"],
        "braid_word":receipt["braid_word"],
        "target_projective_residual":receipt["target_projective_residual"],
        "word_matrix_residual":receipt["word_matrix_residual"],
        "resident_wires":operator.wires,
        "coefficient_fraction_bits":operator.coefficient_fraction_bits,
        "input_amplitude_scale_bits":operator.input_amplitude_scale_bits,
        "mass_before":operator.mass_before.to_string(),
        "mass_after":operator.mass_after.to_string(),
        "gram_before":complex_words(&operator.gram_before),
        "gram_after":complex_words(&operator.gram_after),
        "changed_gram_coordinates":operator.changed_coordinates,
        "inverse_recovery": {
            "same_resident_diagram": true,
            "inverse_matrix":inverse.map(|(re,im)|serde_json::json!({"re":re,"im":im})),
            "applied_input_amplitude_scale_bits":104,
            "recovery_scale_on_gram": "2^104",
            "exact_complex_gram_coordinates":exact_recovered_coordinates,
            "total_complex_gram_coordinates":16,
            "maximum_absolute_integer_coordinate_delta":maximum_recovery_delta.to_string(),
            "signed_complex_coordinate_deltas":complex_words(&recovery_deltas)
        },
        "inverse_mass_after":inverse_operator.mass_after.to_string(),
        "sic_maximum_residual":sic.reconstruction.maximum_residual.to_string(),
        "sic_tolerance":sic.reconstruction.tolerance.to_string(),
        "sic_signed_residuals":complex_words(&sic.signed_residuals),
        "elapsed_seconds":started.elapsed().as_secs_f64()
    });
    println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
    Ok(())
}
