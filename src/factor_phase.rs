//! Prepared exact coherent factor-pair execution, shared with canonical Vox.
use alloc::format;
use alloc::string::String;
use vox_core::fixed_point_quantum_membrane::FixedPointQuantumMembrane;
use vox_core::morphism_factor::{emit_numeral, parse_numeral};
use vox_core::reentry_certificate::{
    certify_reentry, decode_reentry_certificate, encode_reentry_certificate,
    verify_reentry_certificate,
};

pub fn execute_baked() -> Result<String, &'static str> {
    let n = parse_numeral(
        option_env!("FACTOR_PHASE_SOURCE_WORD").unwrap_or("⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋⊙⊡⊣"),
    )
    .map_err(|_| "malformed prepared source")?;
    let rounds = parse_numeral(
        option_env!("FACTOR_PHASE_ROUNDS_WORD").unwrap_or("⊢≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"),
    )
    .map_err(|_| "malformed prepared amplification rounds")?;
    let quantile = parse_numeral(option_env!("FACTOR_PHASE_QUANTILE_WORD").unwrap_or("⊢≻⋈∈⊥∋⊙⊡⊣"))
        .map_err(|_| "malformed prepared measurement quantile")?;
    let denominator =
        parse_numeral(option_env!("FACTOR_PHASE_DENOMINATOR_WORD").unwrap_or("⊢≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"))
            .map_err(|_| "malformed prepared measurement denominator")?;
    let program = FixedPointQuantumMembrane::from_n(&n)?.prepare_structural_execution()?;
    let carrier = program
        .measure_factor_pair(&rounds, &quantile, &denominator)?
        .ok_or("measurement selected an unmarked pair")?;
    let certificate = certify_reentry(&carrier).map_err(|_| "measured carrier failed reentry")?;
    let wire = encode_reentry_certificate(&certificate);
    let decoded =
        decode_reentry_certificate(&wire).map_err(|_| "malformed measured certificate")?;
    verify_reentry_certificate(&decoded).map_err(|_| "measured certificate failed verification")?;
    Ok(format!(
        "{}\n{}\n{}\n{}\n",
        emit_numeral(&n),
        emit_numeral(&carrier.p),
        emit_numeral(&carrier.q),
        wire.iter().collect::<String>()
    ))
}
