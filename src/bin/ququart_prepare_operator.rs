//! Preparation-time contraction of the complete physical Fourier braid.
#[path = "ququart_support/mod.rs"]
mod support;
use std::io::Write;
use num_bigint::{BigInt, BigUint};
use num_traits::Signed;
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
    let mut prepared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    // A supplied operator never bypasses physical contraction in preparation.
    prepared.as_object_mut().ok_or("preparation must be an object")?.remove("prepared_operator");
    let (n, matrix, metrics) = support::contract(&prepared)?;
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
        let base = support::numeral(base_word)?;
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
