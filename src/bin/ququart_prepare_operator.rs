//! Preparation-time contraction of the complete physical Fourier braid.
#[path = "ququart_support/mod.rs"]
mod support;
use std::io::Write;
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, ToPrimitive};
pub fn word(value: &BigUint) -> String {
    g_momonados::godel_calculus::encode_cell_binary(
        &g_momonados::godel_calculus::Nat::from_bits_le(
            (0..value.bits()).map(|bit| value.bit(bit)).collect()))
}
pub fn signed_word(value: &BigInt) -> String {
    format!("{}{}", if value.is_negative() { "≺" } else { "≻" }, word(value.magnitude()))
}


fn prepare() -> Result<String, String> {
    let path = std::env::args().nth(1).ok_or("missing preparation file")?;
    if path == "--numerals" {
        let mut values = serde_json::Map::new();
        for raw in std::env::args().skip(2) {
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                return Err("preparation numeral must be a decimal natural".into());
            }
            let value = BigUint::parse_bytes(raw.as_bytes(), 10).ok_or("invalid decimal natural")?;
            values.insert(raw, serde_json::Value::String(word(&value)));
        }
        return Ok(serde_json::Value::Object(values).to_string());
    }
    if path == "--read-numeral" {
        let raw = std::env::args().nth(2).ok_or("missing canonical numeral word")?;
        return Ok(support::numeral(&raw)?.to_string());
    }
    if path == "--scale-base" || path == "--prepare-powers" {
        let input = std::env::args().nth(2).ok_or("missing source-bound input file")?;
        let prepared: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(input).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let n = support::numeral(prepared["source_word"].as_str().ok_or("missing source word")?)?;
        let base = support::numeral(prepared["base_word"].as_str().ok_or("missing base word")?)?;
        if path == "--scale-base" {
            let scaled = g_momonados::ququart_factor::scaled_modular_base(&n, &base,
                prepared["radix_word"].as_str().ok_or("missing work radix word")?)?;
            return Ok(serde_json::json!({"binary_base_word":word(&base), "base_word":word(&scaled)}).to_string());
        }
        support::validate_base_scaling(&prepared)?;
        let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::prepare(&n, &base)?;
        return Ok(serde_json::json!({
            "controlled_power_words":schedule.powers().iter().map(word).collect::<Vec<_>>(),
            "phase_digits_word":word(&BigUint::from(schedule.powers().len()))}).to_string());
    }
    if path == "--prepare-work" {
        let path = std::env::args().nth(2).ok_or("missing prepared source file")?;
        let prepared = serde_json::from_str(&std::fs::read_to_string(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        return Ok(support::work::compile(&prepared)?.to_string());
    }
    if path == "--radix-words" {
        let extent = std::env::args().nth(2).ok_or("missing radix-ladder extent word")?;
        let extent = support::numeral(&extent)?.to_usize().ok_or("radix ladder exceeds host indexing")?;
        if extent == 0 { return Err("radix ladder requires a nonzero extent word".into()); }
        let words: Vec<_> = (1..=extent).map(|width| word(&(BigUint::from(1u8) << width))).collect();
        return Ok(serde_json::json!({"radix_words": words}).to_string());
    }
    if path == "--defaults" {
        let scalar = |value: u64| word(&BigUint::from(value));
        return Ok(serde_json::json!({
            "seed_word": scalar(1729), "accuracy_word": scalar(4), "radix_word": scalar(4),
            "sk_word": scalar(5), "net_word": scalar(7),
            "capacity_word": scalar(0), "refinement_word": scalar(4),
            "native_arm_word": scalar(1),
        }).to_string());
    }
    if path == "--validate-inputs" {
        let path = std::env::args().nth(2).ok_or("missing input word file")?;
        let inputs: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        let n = support::numeral(inputs["source_word"].as_str().ok_or("missing source word")?)?;
        if n.bits() < 128 { return Err("source word is below the qualifying source precision".into()); }
        for field in ["sk_word", "net_word", "capacity_word", "refinement_word", "accuracy_word"] {
            support::numeral(inputs[field].as_str().ok_or("missing compiler option word")?)?
                .to_usize().ok_or("compiler option exceeds host indexing")?;
        }
        support::numeral(inputs["seed_word"].as_str().ok_or("missing measurement seed word")?)?
            .to_u64().ok_or("measurement seed exceeds the prepared entropy register")?;
        if let Some(base) = inputs["base_word"].as_str() {
            support::validate_base_scaling(&inputs)?;
            g_momonados::ququart_factor::power_of_two_radix_word(
                inputs["radix_word"].as_str().ok_or("missing nested radix word")?)?;
            let base = support::numeral(base)?;
            g_momonados::ququart_factor::QuquartPowerSchedule::prepare(&n, &base)?;
        }
        return Ok("validated source and compiler input words".into());
    }
    let mut prepared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    // A supplied operator never bypasses physical contraction in preparation.
    prepared.as_object_mut().ok_or("preparation must be an object")?.remove("prepared_operator");
    let (n, matrix, metrics) = support::contract(&prepared)?;
    if n.bits() < 128 {
        return Err("qualifying preparations require sources of at least 128 bits".into());
    }
    let format = g_momonados::phase_unbraid::FixedPointFormat::for_modulus(&n)?;
    let entries: Vec<_> = matrix.0.iter().map(|value|
        serde_json::json!({"re_word": signed_word(&value.re), "im_word": signed_word(&value.im)})).collect();
    let scalar = |value: u64| word(&num_bigint::BigUint::from(value));
    let mut operator = serde_json::json!({"w_bits_word": scalar(format.w_bits), "matrix": entries,
        "computational_word": scalar(metrics.computational.to_bits()),
        "leakage_word": scalar(metrics.leakage.to_bits()),
        "closure_word": scalar(metrics.closure.to_bits()),
        "exchanges_word": word(&num_bigint::BigUint::from(metrics.exchanges))});
    if let Some(base_word) = prepared["base_word"].as_str() {
        g_momonados::ququart_factor::power_of_two_radix_word(
            prepared["radix_word"].as_str().ok_or("missing baked nested radix word")?)?;
        let base = support::numeral(base_word)?;
        support::numeral(prepared["seed_word"].as_str().ok_or("missing measurement seed word")?)?
            .to_u64().ok_or("measurement seed word exceeds the prepared entropy register")?;
        let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::prepare(&n, &base)?;
        operator["controlled_power_words"] = serde_json::json!(schedule.powers().iter().map(word).collect::<Vec<_>>());
        operator["phase_digits_word"] = serde_json::Value::String(word(&BigUint::from(schedule.powers().len())));
    }
    Ok(operator.to_string())
}

fn main() {
    match prepare() {
        Ok(report) => {
            if std::io::stdout().lock().write_all(report.as_bytes()).is_err() { std::process::exit(1); }
        }
        Err(error) => { eprintln!("preparation failed: {error}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_preparation_words_preserve_coordinates_and_diagnostic_bits() {
        for value in [BigInt::from(0), BigInt::from(-1), -(BigInt::from(1) << 673usize),
                      (BigInt::from(1) << 674usize) - 1] {
            assert_eq!(support::signed_numeral(&signed_word(&value)).unwrap(), value);
        }
        for bits in [0u64, 1, 0x0010000000000000, 0x3ff0000000000000, u64::MAX] {
            assert_eq!(support::numeral(&word(&BigUint::from(bits))).unwrap(), BigUint::from(bits));
        }
    }
}
