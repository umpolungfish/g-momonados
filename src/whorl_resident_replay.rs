//! Stage 8: replay the readout of a serialized DecisionArena resident snapshot.
//!
//! This checks the actual saved amplitude-graph Gram/SIC reconstruction, not
//! just the arithmetic identity connecting a claimed count to claimed factors.
//! It does NOT establish provenance of the graph or replay the source-row gates.
use crate::anyon_ququart::FixedQuquartSic;
use crate::phase_unbraid::FixedPointFormat;
use crate::ququart_decision::DecisionArena;
use crate::whorl_resident_bridge;
use num_bigint::BigUint;
use num_traits::Zero;
use serde_json::{json, Value};

fn decimal(value: &Value, name: &str) -> Result<BigUint, String> {
    let s = value.as_str().ok_or_else(|| format!("{name} is not a decimal string"))?;
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) ||
        !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{name} is not canonical unsigned decimal"));
    }
    BigUint::parse_bytes(s.as_bytes(), 10)
        .ok_or_else(|| format!("{name} is not a valid integer"))
}

/// Restore a complete four-root resident snapshot and independently recompute
/// its SIC readout. Does not assert that the saved graph was produced by the
/// claimed execution trace, and does not authenticate the serialized bytes.
pub fn verify_snapshot(doc: &Value) -> Result<Value, String> {
    // Stage 7 first verifies the source and factor arithmetic.
    let arithmetic = whorl_resident_bridge::verify_capture(doc)?;
    let report = doc.get("native_report").ok_or("missing native report")?;
    if report.get("schema").and_then(Value::as_str) != Some("retained-square-winding/v2") {
        return Err("full readout replay requires a completed v2 report".into());
    }
    let retained = report.get("retained").ok_or("resident snapshot missing")?;
    if retained.get("checkpoint_path").is_some() {
        return Err("external checkpoint is not embedded: restore its JSON record explicitly before replay".into());
    }
    let (mut arena, roots) = DecisionArena::from_prepared_record_many(retained)?;
    let [prepared, _integer_bank, joint, observed]: [usize; 4] = roots.try_into()
        .map_err(|_| "completed retained snapshot must have exactly four roots".to_string())?;

    let n = decimal(&doc["source"]["N"], "source N")?;
    if arena.mass(prepared) != n {
        return Err("replayed prepared root does not carry the source mass".into());
    }
    if arena.mass(joint) != n {
        return Err("replayed joint root does not carry the source mass".into());
    }

    // Recompute the actual d=4 resident SIC readout, not the separate symbolic
    // d=12 source-identity SIC labels in `source.sic_labels`.
    let zero = arena.zero();
    let gram = arena.control_gram(&[observed, zero, zero, zero]);
    let frame = FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
    let masses = frame.gram_mass_numerators(&gram)?;
    let stated_masses = report["sic_mass_numerators"].as_array()
        .ok_or("missing resident SIC mass numerators")?;
    if stated_masses.len() != masses.len() {
        return Err("resident SIC mass vector has incorrect length".into());
    }
    for (i, mass) in masses.iter().enumerate() {
        if stated_masses[i].as_str() != Some(mass.to_string().as_str()) {
            return Err(format!("resident SIC mass mismatch at index {i}"));
        }
    }
    let recovered = frame.exact_gram_from_masses(&masses)?;
    if recovered.denominator.is_zero() {
        return Err("replayed SIC dual has zero denominator".into());
    }
    if report["sic_denominator"].as_str() != Some(recovered.denominator.to_string().as_str()) {
        return Err("resident SIC denominator mismatch".into());
    }
    let count_numerator = &recovered.numerators[0].0;
    if count_numerator % &recovered.denominator != num_bigint::BigInt::zero() {
        return Err("replayed SIC readout is not integral".into());
    }
    let count = (count_numerator / &recovered.denominator).to_biguint()
        .ok_or("replayed SIC collision count is negative")?;
    let reported_count = decimal(&report["collision_count"], "collision_count")?;
    if count != reported_count {
        return Err("SIC-replayed collision count disagrees with native report".into());
    }
    Ok(json!({
        "valid_arithmetic": arithmetic["valid_arithmetic"],
        "snapshot_reconstruction_valid": true,
        "recomputed_sic_masses_match": true,
        "recomputed_sic_denominator_match": true,
        "recomputed_collision_count": count.to_string(),
        "source_mass": n.to_string(),
        "saved_roots": 4,
        "status": "complete",
        "claims": "restored saved DecisionArena roots and recomputed the SIC readout; no execution-trace provenance or authentication"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_143_snapshot_reconstructs_count() {
        let doc = whorl_resident_bridge::capture(&BigUint::from(143u32), 500_000).unwrap();
        let result = verify_snapshot(&doc).unwrap();
        assert_eq!(result["recomputed_collision_count"], "525");
        assert_eq!(result["snapshot_reconstruction_valid"], true);
    }

    #[test]
    fn altered_sic_mass_is_rejected() {
        let mut doc = whorl_resident_bridge::capture(&BigUint::from(143u32), 500_000).unwrap();
        // Stage 7's arithmetic verifier does not check this field.
        assert!(whorl_resident_bridge::verify_capture(&doc).is_ok());
        doc["native_report"]["sic_mass_numerators"][0] = json!("999999999");
        assert!(whorl_resident_bridge::verify_capture(&doc).is_ok());
        assert!(verify_snapshot(&doc).is_err());
    }

    #[test]
    fn altered_sic_denominator_is_rejected() {
        let mut doc = whorl_resident_bridge::capture(&BigUint::from(143u32), 500_000).unwrap();
        doc["native_report"]["sic_denominator"] = json!("987654321");
        assert!(verify_snapshot(&doc).is_err());
    }

    #[test]
    fn missing_snapshot_is_rejected() {
        let mut doc = whorl_resident_bridge::capture(&BigUint::from(143u32), 500_000).unwrap();
        doc["native_report"].as_object_mut().unwrap().remove("retained");
        assert!(verify_snapshot(&doc).is_err());
    }
}
