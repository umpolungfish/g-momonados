//! Replay terminal phase evidence without executing the factoring membrane.
#[allow(dead_code)]
#[path = "ququart_support/mod.rs"]
mod support;
use g_momonados::{godel_calculus::{check, decode, encode_cell_binary, Operator, Structure}, phase_unbraid::PhaseReadoutAccumulator};
use num_bigint::BigUint;
use num_traits::{One, Zero};

fn numeral(word: &str) -> Result<BigUint, String> {
    let decoded = decode(word).map_err(|e| e.to_string())?;
    if !matches!(decoded.structure, Structure::CellBinary { .. }) {
        return Err("readout value must be an IMASM cell-binary word".into());
    }
    if encode_cell_binary(&decoded.value) != word {
        return Err("readout value is not a canonical IMASM cell-binary word".into());
    }
    Ok(decoded.value.bits_le().iter().enumerate().fold(BigUint::zero(), |value, (bit, set)| {
        if *set { value | (BigUint::one() << bit) } else { value }
    }))
}
fn signed_numeral(word: &str) -> Result<(), String> {
    let body = word
        .strip_prefix('≺')
        .or_else(|| word.strip_prefix('≻'))
        .ok_or("signed prepared coordinate lacks its IMASM direction")?;
    numeral(body).map(|_| ())
}
fn validate_prepared_values(value: &serde_json::Value, key: Option<&str>) -> Result<(), String> {
    match value {
        serde_json::Value::Object(fields) => {
            for (field, child) in fields {
                match field.as_str() {
                    "component" | "telemetry" | "operator_kind" => {
                        if !child.is_string() {
                            return Err(format!("prepared metadata {field} must be a string"));
                        }
                    }
                    _ if field.ends_with("_word") => {
                        let word = child.as_str().ok_or_else(|| format!("prepared {field} must be an IMASM word"))?;
                        if field == "re_word" || field == "im_word" {
                            signed_numeral(word)?;
                        } else {
                            numeral(word)?;
                        }
                    }
                    _ if field.ends_with("_words") => {
                        let words = child.as_array().ok_or_else(|| format!("prepared {field} must be an array of IMASM words"))?;
                        for word in words {
                            let word = word.as_str().ok_or_else(|| format!("prepared {field} contains a non-word value"))?;
                            if field == "exchange_words" { signed_numeral(word)?; }
                            else { numeral(word)?; }
                        }
                    }
                    _ => validate_prepared_values(child, Some(field))?,
                }
            }
            Ok(())
        }
        serde_json::Value::Array(values) => {
            for child in values {
                validate_prepared_values(child, key)?;
            }
            Ok(())
        }
        serde_json::Value::String(_) if matches!(key, Some("component" | "telemetry")) => Ok(()),
        _ => Err(format!("prepared value at {} is not an IMASM word or approved metadata", key.unwrap_or("root"))),
    }
}
fn validate_prepared(path: &str) -> Result<(), String> {
    let prepared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    validate_prepared_values(&prepared, None)?;
    support::validate_base_scaling(&prepared)?;
    if prepared.get("operator_kind").and_then(|value| value.as_str()) == Some("native_fourier") {
        support::contract(&prepared)?;
    }
    if let Some(work) = prepared.get("prepared_work") {
        if support::work::compile(&prepared)? != *work {
            return Err("baked work boundaries differ from source and radix".into());
        }
    }
    if let Some(radix) = prepared.get("radix_word") {
        g_momonados::ququart_factor::power_of_two_radix_word(
            radix.as_str().ok_or("prepared radix must be an IMASM word")?,
        )?;
    }
    Ok(())
}
fn verify() -> Result<bool, String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--validate-prepared" {
        validate_prepared(&args[2])?;
        println!("validated all baked numeric values as IMASM cell-binary words");
        return Ok(false);
    }
    if args.len() != 3 { return Err("usage: ququart_verify_readout prepared.json terminal.stdout".into()); }
    validate_prepared(&args[1])?;
    let prepared: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&args[1]).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    validate_prepared(&args[1])?;
    let terminal = std::fs::read_to_string(&args[2]).map_err(|e| e.to_string())?;
    let mut lines = terminal.lines();
    if lines.next() != Some("completed ququart factor extraction") { return Err("missing completed factor readout".into()); }
    let mut fields = std::collections::BTreeMap::new();
    for line in lines {
        let (key, value) = line.split_once('=').ok_or("terminal readout contains an untyped line")?;
        if fields.insert(key, value).is_some() { return Err("duplicate readout field".into()); }
    }
    let allowed_fields = [
        "source_word", "base_word", "shots_word", "phase_numerator_word",
        "phase_denominator_word", "order_word", "p_word", "q_word",
        "godel_product_verified", "closure_word", "phase_samples",
        "fourier_computational_word", "fourier_leakage_word",
        "fourier_return_word", "fourier_exchanges_word",
        "radix_word", "nested_factor_word",
        "sic_frame_samples",
        "producing_arm_word",
    ];
    if fields.keys().any(|key| !allowed_fields.contains(key)) {
        return Err("terminal output contains an unapproved field or a non-word numeric value".into());
    }
    let field = |key| fields.get(key).copied().ok_or_else(|| format!("missing readout {key}"));
    for key in ["fourier_computational_word", "fourier_leakage_word", "fourier_return_word", "fourier_exchanges_word"] {
        numeral(field(key)?)?;
    }
    let prepared_word = |key: &str| prepared[key].as_str().ok_or_else(|| format!("missing prepared {key}"));
    let n = numeral(prepared_word("source_word")?)?;
    let base = numeral(prepared_word("base_word")?)?;
    if n.bits() < 128 { return Err("factor evidence must be at least 128 bits".into()); }
    if numeral(field("source_word")?)? != n || numeral(field("base_word")?)? != base {
        return Err("terminal input words differ from the prepared IMASM words".into());
    }
    let p = numeral(field("p_word")?)?;
    let q = numeral(field("q_word")?)?;
    if p <= BigUint::one() || q <= BigUint::one() || &p * &q != n { return Err("invalid native factor product".into()); }
    let closure: Vec<_> = field("closure_word")?.split('|').collect();
    if closure.len() != 3 || numeral(closure[0])? != n || numeral(closure[1])? != p || numeral(closure[2])? != q {
        return Err("terminal closure and factor words are different objects".into());
    }
    if field("godel_product_verified")? != "true" {
        return Err("terminal report lacks a successful Gödel product verdict".into());
    }
    if let Some(radix) = prepared.get("radix_word") {
        let radix = radix.as_str().ok_or("baked radix must be an IMASM word")?;
        if field("radix_word")? != radix || field("nested_factor_word")? != field("closure_word")? {
            return Err("terminal nested factor carrier differs from its baked radix or closure".into());
        }
        let (nested_p, nested_q) = g_momonados::ququart_factor::nested_radix_factor_words(
            prepared_word("source_word")?, field("p_word")?, field("q_word")?, radix,
        )?;
        if nested_p != field("p_word")? || nested_q != field("q_word")? {
            return Err("terminal factors differ from the nested meeting point".into());
        }
    }
    let source_word = prepared_word("source_word")?;
    if !check(closure[1], Operator::Mul, closure[2], source_word).map_err(|e| e.to_string())?.valid {
        return Err("terminal factors do not close through Gödel multiplication".into());
    }
    if let Some(arm) = fields.get("producing_arm_word") {
        let arm = numeral(arm)?;
        if arm.is_zero() {
            if prepared.get("native_arm_word").and_then(|v|v.as_str()).map(numeral).transpose()? != Some(BigUint::one()) {
                return Err("native factor arm was not included in the baked membrane".into());
            }
            if !numeral(field("shots_word")?)?.is_zero() || field("phase_samples")? != "[]"
                || field("sic_frame_samples")? != "[]" || fields.contains_key("order_word")
                || fields.contains_key("phase_numerator_word") || fields.contains_key("phase_denominator_word") {
                return Err("native factor report claims phase measurements it did not produce".into());
            }
            return Ok(true);
        }
        if arm != BigUint::one() { return Err("unknown producing arm word".into()); }
    }
    let samples: serde_json::Value = serde_json::from_str(field("phase_samples")?).map_err(|e| e.to_string())?;
    validate_prepared_values(&samples, None)?;
    let samples = samples.as_array().ok_or("phase samples must be an array")?;
    if numeral(field("shots_word")?)? != BigUint::from(samples.len()) {
        return Err("shot-count word differs from resident phase ledger".into());
    }
    let digits = numeral(prepared["prepared_operator"]["phase_digits_word"].as_str().ok_or("missing prepared phase resolution")?)?;
    if digits != BigUint::from(n.bits() + 4) { return Err("prepared phase resolution differs from source".into()); }
    let denominator = BigUint::one() << usize::try_from(2 * (n.bits() + 4)).map_err(|_| "phase resolution exceeds host indexing")?;
    if let Some(raw) = fields.get("sic_frame_samples") {
        let witnesses: serde_json::Value = serde_json::from_str(raw).map_err(|e|e.to_string())?;
        validate_prepared_values(&witnesses,None)?;
        let witnesses = witnesses.as_array().ok_or("SIC frame ledger must be an array")?;
        let per_shot = usize::try_from(n.bits()+4).map_err(|_| "SIC ledger width exceeds host indexing")?;
        if witnesses.len() != samples.len().checked_mul(per_shot).ok_or("SIC ledger length overflow")? {
            return Err("SIC frame ledger differs from the measured phase stages".into());
        }
        let format = g_momonados::phase_unbraid::FixedPointFormat::for_modulus(&n)?;
        let sic = g_momonados::anyon_ququart::FixedQuquartSic::new(&format)?;
        for (index,witness) in witnesses.iter().enumerate() {
            let entries = witness["gram"].as_array().ok_or("missing SIC control Gram")?;
            let gram: [(num_bigint::BigInt,num_bigint::BigInt);16] = entries.iter().map(|entry| {
                Ok((support::signed_numeral(entry["re_word"].as_str().ok_or("missing Gram real word")?)?,
                    support::signed_numeral(entry["im_word"].as_str().ok_or("missing Gram imaginary word")?)?))
            }).collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_| "SIC Gram must contain every control pair")?;
            for row in 0..4 {
                for col in 0..4 {
                    if gram[4*row+col].0 != gram[4*col+row].0 || gram[4*row+col].1 != -&gram[4*col+row].1 {
                        return Err("SIC control Gram is not Hermitian".into());
                    }
                }
            }
            let masses: [BigUint;16] = witness["mass_words"].as_array().ok_or("missing SIC frame masses")?
                .iter().map(|value| numeral(value.as_str().ok_or("SIC mass must be a word")?))
                .collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_| "SIC frame must retain every outcome")?;
            if sic.gram_masses(&gram)? != masses { return Err("SIC masses differ from their measured control Gram".into()); }
            let reconstruction = sic.certify_gram_frame(&gram,&masses)?;
            // Earlier ledgers retain their original fields. When a ledger
            // carries the complete reconstruction certificate, bind every
            // entry and both residual words to the recomputed certificate.
            if witness.get("recovered_gram").is_some()
                || witness.get("reconstruction_residual_word").is_some()
                || witness.get("reconstruction_tolerance_word").is_some() {
                let entries = witness["recovered_gram"].as_array().ok_or("missing reconstructed SIC Gram")?;
                let recovered: [(num_bigint::BigInt,num_bigint::BigInt);16] = entries.iter().map(|entry| {
                    Ok((support::signed_numeral(entry["re_word"].as_str().ok_or("missing reconstructed real word")?)?,
                        support::signed_numeral(entry["im_word"].as_str().ok_or("missing reconstructed imaginary word")?)?))
                }).collect::<Result<Vec<_>,String>>()?.try_into().map_err(|_| "reconstructed SIC Gram must contain every control pair")?;
                if recovered != reconstruction.recovered
                    || numeral(witness["reconstruction_residual_word"].as_str().ok_or("missing reconstruction residual word")?)?
                        != *reconstruction.maximum_residual.magnitude()
                    || numeral(witness["reconstruction_tolerance_word"].as_str().ok_or("missing reconstruction tolerance word")?)?
                        != *reconstruction.tolerance.magnitude() {
                    return Err("SIC reconstruction certificate differs from the measured control Gram".into());
                }
            }
            let sample = &samples[index/per_shot];
            let phase = numeral(sample["numerator_word"].as_str().ok_or("missing measured phase numerator")?)?;
            let expected = (phase >> (2*(index%per_shot))) & BigUint::from(3u8);
            if numeral(witness["digit_word"].as_str().ok_or("missing SIC boundary digit word")?)? != expected {
                return Err("SIC boundary digit differs from the measured phase ledger".into());
            }
        }
    } else if prepared.get("binary_base_word").is_some() {
        return Err("scaled-base membrane lacks its inclusive SIC frame ledger".into());
    }
    let mut accumulator = PhaseReadoutAccumulator::default();
    let mut closed = None;
    for (index, sample) in samples.iter().enumerate() {
        let k = numeral(sample["numerator_word"].as_str().ok_or("missing measured numerator word")?)?;
        let m = numeral(sample["denominator_word"].as_str().ok_or("missing measured denominator word")?)?;
        if m != denominator { return Err("measured phase resolution differs from compiled schedule".into()); }
        closed = accumulator.close_fraction(&k, &m, &base, &n)?;
        if closed.is_some() && index + 1 != samples.len() { return Err("readout continued after certified extraction".into()); }
    }
    let (order, left, right) = closed.ok_or("recorded measurements do not close factor extraction")?;
    if !((left == p && right == q) || (left == q && right == p)) { return Err("measured phase yields different factor arms".into()); }
    if numeral(field("order_word")?)? != order { return Err("reported order word differs from measured phase closure".into()); }
    let last = samples.last().ok_or("phase sample ledger is empty")?;
    if numeral(field("phase_numerator_word")?)? != numeral(last["numerator_word"].as_str().ok_or("missing final phase numerator word")?)?
        || numeral(field("phase_denominator_word")?)? != numeral(last["denominator_word"].as_str().ok_or("missing final phase denominator word")?)? {
        return Err("terminal phase words differ from the final resident phase sample".into());
    }
    Ok(true)
}
fn main() {
    match verify() {
        Ok(true) => println!("verified terminal producing arm and Gödel factor product"),
        Ok(false) => {},
        Err(error) => { eprintln!("readout verification failed: {error}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(value: u8) -> String {
        encode_cell_binary(&g_momonados::godel_calculus::Nat::from_bits_le(
            (0..8).map(|bit| value & (1 << bit) != 0).collect(),
        ))
    }

    #[test]
    fn prepared_validator_requires_canonical_words_for_every_numeric_leaf() {
        let prepared = serde_json::json!({
            "component": "ququart_factor",
            "telemetry": "terminal_only",
            "source_word": word(15),
            "base_word": word(2),
            "seed_word": word(7),
            "prepared_operator": {
                "w_bits_word": word(32),
                "controlled_power_words": [word(2), word(4)],
                "phase_digits_word": word(2),
                "matrix": [{"re_word": format!("≻{}", word(1)), "im_word": format!("≺{}", word(0))}]
            }
        });
        assert!(validate_prepared_values(&prepared, None).is_ok());
        let mut raw_numeric = prepared.clone();
        raw_numeric["seed"] = serde_json::json!(7);
        assert!(validate_prepared_values(&raw_numeric, None).is_err());
        let mut decimal_text = prepared;
        decimal_text["base_word"] = serde_json::json!("2");
        assert!(validate_prepared_values(&decimal_text, None).is_err());
    }
}
