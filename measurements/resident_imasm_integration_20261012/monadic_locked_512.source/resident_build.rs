//! Cargo membrane construction, using the same modules as the executable.
use crate::resident_work as work;
use g_momonados::{resident_glut::ResidentGlut,word_tape::WordTape};
use std::time::Instant;

pub fn compile(source_text: &str) -> Result<serde_json::Value, String> {
    let source = WordTape::from_canonical_word(source_text.trim()).ok_or("invalid source word")?;
    let preparation = Instant::now();
    let mut resident = ResidentGlut::from_word(source.as_word())?;
    let witness = resident.sic_witness()?;
    eprintln!("prepared-SIC residual={} tolerance={} seconds={:.6}",witness.reconstruction.maximum_residual,
        witness.reconstruction.tolerance,preparation.elapsed().as_secs_f64());
    let mut carrier = resident.into_source_whole_register_phase_observed(|stage|
        eprintln!("preparation-stage {stage} seconds={:.6}",preparation.elapsed().as_secs_f64()))?;
    let clock_export: serde_json::Value = serde_json::from_str(include_str!(
        "measurements/membrane_reentry_20261010/certified_clocks.json")).map_err(|e| e.to_string())?;
    // The period-two oscillator is a fixed program constant, with no instance
    // witness. Its executed ticks order complete prepared modular lanes.
    let clock = clock_export["clocks"].as_array().ok_or("missing certified clock export")?.iter()
        .find(|r| r["retained"].as_u64()==Some(0) && r["seed"].as_u64()==Some(1))
        .ok_or("missing certified alternating clock")?;
    carrier.bind_certified_reentry(clock)?;
    let mut record = carrier.prepared_record()?;
    let mut power = WordTape::from_small(2);
    let mut powers = Vec::with_capacity(source.bit_len());
    for _ in 0..source.bit_len() {
        powers.push(power.as_word().to_owned());
        let square = power.mul(&power).divmod(&source).unwrap().1;
        power = square.mul(&square).divmod(&source).unwrap().1;
    }
    let mut program = serde_json::json!({"source_word":source.as_word(),"base_word":WordTape::from_small(2).as_word(),
        "radix_word":WordTape::one_at(8).as_word(),"prepared_operator":{
            "controlled_power_words":powers,"phase_digits_word":WordTape::from_small(source.bit_len() as u64).as_word()}});
    program["prepared_work"] = work::compile(&program)?;
    work::decode(&program)?;
    eprintln!("prepared-modular-program seconds={:.6}",preparation.elapsed().as_secs_f64());
    record["program"] = program;
    record["preparation_seconds"] = serde_json::json!(preparation.elapsed().as_secs_f64());
    record["preparation_nanoseconds"] = serde_json::json!(
        u64::try_from(preparation.elapsed().as_nanos()).map_err(|_| "preparation clock overflow")?);
    record["SIC_signed_residuals"] = serde_json::json!(witness.signed_residuals.iter().map(|(re,im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>());
    record["SIC_rational_denominator_word"] = serde_json::json!(signed_word(&witness.rational_denominator));
    record["SIC_rational_residual_words"] = serde_json::json!(witness.rational_signed_residuals.iter().map(|(re,im)|
        serde_json::json!({"re_word":signed_word(re),"im_word":signed_word(im)})).collect::<Vec<_>>());
    Ok(record)
}

fn signed_word(value: &num_bigint::BigInt) -> String {
    use num_traits::Signed;
    format!("{}{}",if value.is_negative() { "≺" } else { "≻" },work::word_for_record(value.magnitude()))
}
