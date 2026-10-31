//! Stage 9: deterministic source-to-snapshot execution replay.
//!
//! Regenerates the source-bound DecisionArena from N through the *actual*
//! source_braid_state::execute_with_node_budget path. Compares the complete
//! serialized native report, including every root of its retained graph,
//! after removing ONLY execution timing fields. Unlike a Gram-only comparison,
//! this requires full structural equality of the saved amplitude graph.
//!
//! Security boundary: same implementation, same build/environment, no external
//! attestation, no independently verified gate semantics, no authenticated
//! origin. A node budget is tested between operations, not per allocation.
use crate::source_braid_state;
use crate::word_tape::WordTape;
use crate::whorl_resident_bridge;
use crate::whorl_resident_replay;
use num_bigint::BigUint;
use num_traits::One;
use serde_json::{json, Value};

/// These are precisely the wall-clock fields emitted by the supplied
/// source_braid_state.rs, not contents of the retained amplitude graph.
fn volatile_field(container: &str, key: &str) -> bool {
    (container == "native_report" && matches!(key,
        "preparation_seconds" | "reduction_seconds" | "contraction_seconds" |
        "readout_and_factor_seconds" | "seconds" | "seconds_before_serialization")) ||
    (key == "seconds" && container.starts_with("native_report.source_rows[") &&
     container.ends_with(']'))
}

/// Compare the native reports without allocating duplicate copies of the full
/// retained graph. Includes the full serialized graph, not just its Gram.
/// Report the first mismatch path, without printing large amplitude arrays.
fn first_difference(expected: &Value, observed: &Value, at: &str) -> Option<String> {
    if expected == observed { return None; }
    match (expected, observed) {
        (Value::Object(left), Value::Object(right)) => {
            for key in left.keys() {
                if volatile_field(at, key) {continue;}
                let place = format!("{at}.{key}");
                match right.get(key) {
                    None => return Some(format!("{place} is absent from regenerated report")),
                    Some(other) => {
                        if let Some(mismatch) = first_difference(left.get(key).expect("left map key disappeared"), other, &place) {
                            return Some(mismatch);
                        }
                    }
                }
            }
            for key in right.keys() {
                if !volatile_field(at, key) && !left.contains_key(key) {
                    return Some(format!("{at}.{key} is absent from saved report"));
                }
            }
            None
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!("{at} length differs: saved={}, replayed={}",
                                    left.len(), right.len()));
            }
            for (i, (a, b)) in left.iter().zip(right).enumerate() {
                if let Some(mismatch) = first_difference(a, b, &format!("{at}[{i}]")) {
                    return Some(mismatch);
                }
            }
            None
        }
        _ => Some(format!("{at} differs")),
    }
}

fn source_number(doc: &Value) -> Result<BigUint, String> {
    let s = doc.get("source").and_then(|v| v.get("N"))
        .and_then(Value::as_str).ok_or("source.N is missing")?;
    if s.is_empty() || (s.starts_with('0') && s.len() > 1) ||
        !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err("source.N must be canonical unsigned decimal".into());
    }
    let n = BigUint::parse_bytes(s.as_bytes(), 10).ok_or("invalid source.N")?;
    if n <= BigUint::one() {return Err("source.N must exceed one".into());}
    Ok(n)
}

/// Reexecute the complete native source program, then compare *all* stable
/// fields of the regenerated report against the saved report. The reported
/// `node_budget` for complete Stage 7 records was not preserved; the budget
/// passed here is a REPLAY budget, not an attestation of the capture budget.
///
/// `max_bits` explicitly limits inadvertent expensive replay for untrusted N.
/// Requires an embedded snapshot; does not follow file paths from the input.
pub fn verify_execution(doc: &Value, budget: usize, max_bits: u64) -> Result<Value, String> {
    if budget == 0 {return Err("replay budget must be positive".into());}
    if max_bits == 0 {return Err("max_bits must be positive".into());}
    if std::env::var_os("SOURCE_BRAID_CHECKPOINT_PATH").is_some() {
        return Err("unset SOURCE_BRAID_CHECKPOINT_PATH: replay requires embedded snapshots and must not write external checkpoints".into());
    }
    // Validate identity/lanes/SIC and arithmetic before allocating a new DAG.
    let arithmetic = whorl_resident_bridge::verify_capture(doc)?;
    let n = source_number(doc)?;
    if n.bits() > max_bits {
        return Err(format!("source has {} bits; --max-bits limits replay to {max_bits}",n.bits()));
    }
    let saved = doc.get("native_report").ok_or("native_report is missing")?;
    let held = saved.get("schema").and_then(Value::as_str)
        == Some("retained-square-winding-held/v1");
    if !held {
        // Stage 8 independently checks the saved graph's SIC reconstruction
        // before Stage 9 regenerates its actual source program.
        whorl_resident_replay::verify_snapshot(doc)?;
    } else if saved.get("retained").is_none() {
        return Err("held report is missing its embedded snapshot".into());
    }
    if saved.get("retained").and_then(|v| v.get("checkpoint_path")).is_some() {
        return Err("external retained checkpoint is not supported".into());
    }

    let regenerated = source_braid_state::execute_with_node_budget(
        WordTape::from_biguint(&n), budget)?;
    if regenerated.get("retained").and_then(|v| v.get("checkpoint_path")).is_some() {
        return Err("regenerated resident state was not embedded".into());
    }
    if let Some(mismatch) = first_difference(saved, &regenerated, "native_report") {
        return Err(format!("execution replay mismatch: {mismatch}"));
    }
    Ok(json!({
        "status": if held {"held"} else {"complete"},
        "valid_arithmetic": arithmetic["valid_arithmetic"],
        "replayed_source": n.to_string(),
        "replayed_source_bits": n.bits(),
        "replay_node_budget": budget,
        "recorded_capture_budget_verified": false,
        "replayed_rows": regenerated["source_rows"].as_array().map(|r| r.len()).unwrap_or(0),
        "retained_snapshot_exact_match": true,
        "full_deterministic_native_report_match": true,
        "ignored_report_fields": ["preparation_seconds", "reduction_seconds",
            "contraction_seconds", "readout_and_factor_seconds", "seconds",
            "seconds_before_serialization", "source_rows[*].seconds"],
        "recomputed_collision_count": if held {Value::Null} else {regenerated["collision_count"].clone()},
        "factor_verified": arithmetic["factor_verified"],
        "claims": "same-implementation deterministic reexecution and full serialized graph comparison; no authenticated origin, source-level formal verification, or independent gate implementation"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::whorl_resident_bridge;

    fn fixture() -> Value {
        whorl_resident_bridge::capture(&BigUint::from(143u16), 500_000).unwrap()
    }
    fn verify(doc: &Value) -> Result<Value,String> {
        verify_execution(doc,500_000,64)
    }

    #[test]
    fn deterministic_143_replays_entire_graph() {
        let response = verify(&fixture()).unwrap();
        assert_eq!(response["retained_snapshot_exact_match"], true);
        assert_eq!(response["replayed_rows"], 8);
        assert_eq!(response["recomputed_collision_count"], "525");
    }
    #[test]
    fn changing_only_timings_is_allowed() {
        let mut doc=fixture();
        doc["native_report"]["source_rows"][0]["seconds"] = json!(9876.5);
        doc["native_report"]["seconds"] = json!(-1.0);
        assert!(verify(&doc).is_ok());
    }
    #[test]
    fn changing_row_node_count_is_rejected() {
        let mut doc=fixture();
        doc["native_report"]["source_rows"][0]["nodes"] = json!(999);
        let error=verify(&doc).unwrap_err();
        assert!(error.contains("source_rows[0].nodes"),"{error}");
    }
    #[test]
    fn changing_execution_tape_is_rejected() {
        let mut doc=fixture();
        doc["native_report"]["execution_tape"] = json!("⊢⊣");
        assert!(verify(&doc).unwrap_err().contains("execution_tape"));
    }
    #[test]
    fn changing_tape_deposits_is_rejected() {
        let mut doc=fixture();
        doc["native_report"]["tape_deposits"] = json!([12345]);
        assert!(verify(&doc).unwrap_err().contains("tape_deposits"));
    }
    #[test]
    fn invalid_sic_masses_rejected_before_replay() {
        let mut doc=fixture();
        doc["native_report"]["sic_mass_numerators"][0] = json!("987654321");
        assert!(verify(&doc).is_err());
    }
    #[test]
    fn cap_prevents_excessive_replay() {
        let doc=fixture();
        assert!(verify_execution(&doc,500_000,7).unwrap_err().contains("max-bits"));
        assert!(verify_execution(&doc,0,64).is_err());
    }
    #[test]
    fn held_capture_replays_as_held() {
        let doc=whorl_resident_bridge::capture(&BigUint::from(143u16), 1).unwrap();
        assert_eq!(doc["native_report"]["status"],"held");
        let result=verify_execution(&doc,1,64).unwrap();
        assert_eq!(result["status"],"held");
        assert_eq!(result["retained_snapshot_exact_match"],true);
    }
}
