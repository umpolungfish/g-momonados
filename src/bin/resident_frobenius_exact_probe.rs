//! Audit exact adjugate recovery for the compiled circuit matrix on one N-only resident diagram.
use g_momonados::{resident_glut::ResidentGlut, word_tape::WordTape};
use num_bigint::BigInt;
use std::{env, fs, time::Instant};

fn main() -> Result<(), String> {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept();
    let mut args = env::args().skip(1);
    let source_path = args.next().ok_or("usage: resident_frobenius_exact_probe N_ONLY_IMASM OPERATOR_JSON")?;
    let operator_path = args.next().ok_or("missing circuit operator receipt")?;
    if args.next().is_some() { return Err("expected exactly two input paths".into()); }
    let source_text = fs::read_to_string(&source_path).map_err(|e| e.to_string())?;
    let source = WordTape::from_canonical_word(source_text.trim()).ok_or("invalid N-only IMASM source")?;
    if source.bit_len() < 200 { return Err("resident Frobenius probe requires a balanced-source domain input of at least 200 bits".into()); }
    let receipt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&operator_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let entries = receipt["synthesized_matrix"].as_array().ok_or("operator matrix is missing")?;
    if entries.len() != 4 || !receipt["unitary"].as_bool().unwrap_or(false)
        || receipt["word_matrix_residual"].as_f64().unwrap_or(f64::INFINITY) >= 1e-6 {
        return Err("operator receipt lacks a passing split/fuse word check".into());
    }
    let scale = (1u64 << 52) as f64;
    let matrix: [(BigInt, BigInt); 4] = core::array::from_fn(|i| {
        (BigInt::from((entries[i]["re"].as_f64().unwrap() * scale).round() as i64),
         BigInt::from((entries[i]["im"].as_f64().unwrap() * scale).round() as i64))
    });
    let started = Instant::now();
    let mut resident = ResidentGlut::from_word(source.as_word())?;
    let witness = resident.audit_circuit_braid_dual(&matrix, 52)?;
    let sic = resident.sic_witness()?;
    let report = serde_json::json!({
        "instrument": "resident exact-numerator matrix-dual round trip",
        "source_path": source_path,
        "source_bits": source.bit_len(),
        "operator_path": operator_path,
        "circuit": receipt["circuit"],
        "braid_word": receipt["braid_word"],
        "target_projective_residual": receipt["target_projective_residual"],
        "word_matrix_residual": receipt["word_matrix_residual"],
        "same_resident_diagram": true,
        "matrix_denominator": "2^52",
        "dual": "Gaussian-integer adjugate with exact determinant normalization",
        "determinant": {"re": witness.determinant.0.to_string(), "im": witness.determinant.1.to_string()},
        "exact_full_state_return_after_determinant_normalization": witness.exact_full_state_return,
        "exact_prepared_state_restoration": witness.exact_prepared_state_restoration,
        "exact_complex_gram_coordinates": witness.exact_complex_gram_coordinates,
        "total_complex_gram_coordinates": witness.total_complex_gram_coordinates,
        "maximum_absolute_gram_delta": witness.maximum_absolute_gram_delta.to_string(),
        "exact_mass_return_after_determinant_normalization": witness.exact_mass_return,
        "mass_delta": witness.mass_delta.to_string(),
        "forward_frame":witness.forward_frame.record(),
        "reverse_frame":witness.reverse_frame.record(),
        "sic_maximum_residual": sic.reconstruction.maximum_residual.to_string(),
        "sic_tolerance": sic.reconstruction.tolerance.to_string(),
        "elapsed_seconds": started.elapsed().as_secs_f64()
    });
    println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
    Ok(())
}
