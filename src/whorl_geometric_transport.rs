//! Stage 10: exact, source-preserving peripheral transport with a SnapPy evidence boundary.
//!
//! The Rust runtime checks the structure, ordered integral transports, and
//! source recoverability of an m137 surgery certificate. It does NOT calculate
//! hyperbolic isometries: the companion Sage/SnapPy rechecker does that.
//! The saved four source IDs and their full framing/surgery descriptions are
//! never replaced by a single manifold-signature key.
use serde_json::{json, Value};
use std::collections::HashSet;

type Mat = [[i128; 2]; 2];
type Slope = [i128; 2];
const I: Mat = [[1, 0], [0, 1]];
const M16: Mat = [[-47, 256], [-9, 49]];
const M13: Mat = [[-38, 169], [-9, 40]];
const SCHEMA: &str = "prime-whorl-geometric-transport-v1";

fn matrix(name: &str) -> Result<Mat, String> {
    match name {
        "M16" => Ok(M16),
        "M13" => Ok(M13),
        _ => Err(format!("unsupported winding operator '{name}' (expected M16 or M13)")),
    }
}
fn pair(value: &Value, at: &str) -> Result<Slope, String> {
    let values = value.as_array().ok_or_else(|| format!("{at} is not a pair"))?;
    if values.len() != 2 {return Err(format!("{at} needs two integers"));}
    let a = values[0].as_i64().ok_or_else(|| format!("{at}[0] must be an integer"))?;
    let b = values[1].as_i64().ok_or_else(|| format!("{at}[1] must be an integer"))?;
    Ok([i128::from(a), i128::from(b)])
}
fn multiply(a: Mat, b: Mat) -> Result<Mat, String> {
    let mut r = [[0i128; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            let p = a[i][0].checked_mul(b[0][j]).ok_or("matrix multiplication overflow")?;
            let q = a[i][1].checked_mul(b[1][j]).ok_or("matrix multiplication overflow")?;
            r[i][j] = p.checked_add(q).ok_or("matrix addition overflow")?;
        }
    }
    Ok(r)
}
fn apply(a: Mat, v: Slope) -> Result<Slope, String> {
    let mut r = [0i128; 2];
    for i in 0..2 {
        let p = a[i][0].checked_mul(v[0]).ok_or("slope multiplication overflow")?;
        let q = a[i][1].checked_mul(v[1]).ok_or("slope multiplication overflow")?;
        r[i] = p.checked_add(q).ok_or("slope addition overflow")?;
    }
    Ok(r)
}
fn invert(a: Mat) -> Result<Mat, String> {
    let d = a[0][0].checked_mul(a[1][1]).ok_or("determinant overflow")?
        .checked_sub(a[0][1].checked_mul(a[1][0]).ok_or("determinant overflow")?)
        .ok_or("determinant overflow")?;
    if d != 1 && d != -1 {return Err("winding matrix must have determinant ±1".into());}
    Ok([[a[1][1] / d, -a[0][1] / d], [-a[1][0] / d, a[0][0] / d]])
}
fn nonempty<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} must be a nonempty string"))
}
fn expected_sources() -> HashSet<String> {
    let mut result = HashSet::new();
    for link in ["L7a5", "L7a6"] {
        for cusp in 0..2 {
            result.insert(format!("{link}:cusp{cusp}:slope0,1"));
        }
    }
    result
}
fn source<'a>(cert: &'a Value, id: &str) -> Result<&'a Value, String> {
    cert.get("sources").and_then(Value::as_array)
        .ok_or("missing sources array")?
        .iter()
        .find(|entry| entry.get("source_id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("unknown retained source ID: {id}"))
}
/// Verify that all four surgery inputs survive and the recorded passive and
/// active slopes are precisely related to their original peripheral framings.
/// Geometry signatures/statuses are evidence ASSERTIONS from the Sage producer;
/// they are independently recomputed by geometry_certificate.py recheck.
pub fn verify_certificate(cert: &Value) -> Result<Value, String> {
    if cert.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err("unsupported geometric transport schema".into());
    }
    let target = cert.get("target").ok_or("missing target")?;
    if target.get("manifold").and_then(Value::as_str) != Some("m137") {
        return Err("expected m137 target".into());
    }
    if target.get("verified_signatures_requested").and_then(Value::as_bool) != Some(true) {
        return Err("verified SnapPy signatures not requested in evidence".into());
    }
    let signature = nonempty(target, "isometry_signature")?;
    let sources = cert.get("sources").and_then(Value::as_array).ok_or("missing sources")?;
    if sources.len() != 4 {return Err("expected exactly four surgery sources".into());}
    let expected = expected_sources();
    let mut seen = HashSet::new();
    for entry in sources {
        let link = nonempty(entry, "link")?;
        let cusp = entry.get("filled_cusp").and_then(Value::as_u64)
            .ok_or("missing/invalid filled_cusp")?;
        let slope = pair(&entry["slope"], "slope")?;
        if slope != [0, 1] || cusp > 1 {
            return Err(format!("unsupported source framing: {link}, cusp {cusp}"));
        }
        let id = format!("{link}:cusp{cusp}:slope{},{}", slope[0], slope[1]);
        if !expected.contains(&id) || !seen.insert(id.clone()) ||
            entry.get("source_id").and_then(Value::as_str) != Some(id.as_str()) {
            return Err(format!("duplicate, missing, or noncanonical source identity: {id}"));
        }
        if entry.get("framing").and_then(Value::as_str) !=
            Some("SnapPy default peripheral basis") {
            return Err(format!("source framing is not canonical: {id}"));
        }
        if entry.get("isometry_signature").and_then(Value::as_str) != Some(signature) {
            return Err(format!("source signature differs from m137: {id}"));
        }
        let controls = entry.get("controls").and_then(Value::as_object)
            .ok_or_else(|| format!("missing controls for {id}"))?;
        if controls.len() != 2 {return Err(format!("expected two controls for {id}"));}
        for name in ["M16", "M13"] {
            let mat = matrix(name)?;
            let saved = controls.get(name).ok_or_else(|| format!("missing {name} control"))?;
            if saved.get("matrix") != Some(&json!(mat)) {
                return Err(format!("{id}: {name} matrix tampered"));
            }
            let expected_passive = apply(invert(mat)?, slope)?;
            let expected_active = apply(mat, slope)?;
            if pair(&saved["passive_slope"], "passive_slope")? != expected_passive ||
               pair(&saved["active_slope"], "active_slope")? != expected_active {
                return Err(format!("{id}: {name} slope transport mismatch"));
            }
            if saved.get("passive_status").and_then(Value::as_str) != Some("ISOMETRIC") ||
               saved.get("active_status").and_then(Value::as_str) != Some("NOT_ISOMETRIC") {
                return Err(format!("{id}: {name} observed control not satisfied"));
            }
            if apply(mat, expected_passive)? != slope {
                return Err(format!("{id}: {name} inverse transport failed"));
            }
        }
    }
    if seen != expected {return Err("source presentation set incomplete".into());}
    let resident = cert.get("resident").ok_or("missing resident replay binding")?;
    if resident.get("source_N").and_then(Value::as_str) != Some("143") ||
       resident.get("collision_count").and_then(Value::as_str) != Some("525") ||
       resident.get("factor_verified").and_then(Value::as_bool) != Some(true) ||
       resident.get("stage9_status").and_then(Value::as_str) != Some("complete") ||
       resident.get("full_report_match").and_then(Value::as_bool) != Some(true) ||
       resident.get("snapshot_match").and_then(Value::as_bool) != Some(true) {
        return Err("resident Stage 9 result binding invalid".into());
    }
    let hash = nonempty(resident, "certificate_sha256")?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("resident SHA-256 must be 64 lowercase hex digits".into());
    }
    Ok(json!({"schema":"prime-whorl-geometric-native-verification-v1",
        "status":"complete", "target_manifold":"m137", "retained_source_presentations":4,
        "all_saved_signatures_equal_target":true, "passive_slope_transport_verified":8,
        "active_slope_transport_verified":8,"retained_source_ids_unique":true,
        "source_recovery_available":true,"resident_stage9_result_bound":true,
        "claims":"native exact slope/word algebra and source preservation; imported SnapPy observations and resident replay are checked for schema and consistency, not independently recomputed by this Rust command"}))
}

/// Return the EXACT saved source description, not a reconstruction guessed
/// from the common manifold signature.
pub fn recover_source(cert: &Value, id: &str) -> Result<Value, String> {
    verify_certificate(cert)?;
    Ok(source(cert, id)?.clone())
}

/// Ordered operation word: first operator acts first (column-vector convention).
/// Computes the active slope on the original framing and passive coordinates
/// for an equivalent framed surgery. No new geometric isometry is asserted.
pub fn transport_word(cert: &Value, id: &str, word: &str) -> Result<Value, String> {
    verify_certificate(cert)?;
    let entry = source(cert, id)?;
    let slope = pair(&entry["slope"], "slope")?;
    let ops: Vec<&str> = word.split(',').collect();
    if ops.is_empty() || ops.len() > 6 || ops.iter().any(|s| s.is_empty()) {
        return Err("--word requires 1..6 comma-separated M16/M13 operators".into());
    }
    let mut product = I;
    for name in &ops {
        product = multiply(matrix(name)?, product)?;
    }
    let active = apply(product, slope)?;
    let passive = apply(invert(product)?, slope)?;
    if apply(product, passive)? != slope {
        return Err("passive reconstruction failed".into());
    }
    Ok(json!({"schema":"prime-whorl-geometric-transport-result-v1",
        "source_id":id,"manifold_signature":cert["target"]["isometry_signature"],
        "ordered_word":ops,"total_matrix":product,"original_slope":slope,
        "active_slope_on_original_framing":active,
        "passive_slope_coordinates_after_rebasing":passive,
        "passive_roundtrip_recovers_original":true,
        "retained_source":entry,
        "claims":"exact ordered integral slope transport and complete retained source; multisymbol-word geometric evaluations are not asserted without a separate SnapPy check"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        let mut sources = Vec::new();
        for link in ["L7a5", "L7a6"] {
            for cusp in 0..2 {
                let mut controls = serde_json::Map::new();
                for (name, mat) in [("M16", M16), ("M13", M13)] {
                    controls.insert(name.to_string(), json!({
                        "matrix":mat,
                        "passive_slope":apply(invert(mat).unwrap(),[0,1]).unwrap(),
                        "active_slope":apply(mat,[0,1]).unwrap(),
                        "passive_status":"ISOMETRIC","active_status":"NOT_ISOMETRIC"}));
                }
                sources.push(json!({"link":link,"filled_cusp":cusp,"slope":[0,1],
                    "framing":"SnapPy default peripheral basis",
                    "source_id":format!("{link}:cusp{cusp}:slope0,1"),
                    "isometry_signature":"signature-TEST", "controls":controls}));
            }
        }
        json!({"schema":SCHEMA,"target":{"manifold":"m137",
            "verified_signatures_requested":true,"isometry_signature":"signature-TEST"},
            "sources":sources,"resident":{"source_N":"143","collision_count":"525",
            "factor_verified":true,"stage9_status":"complete",
            "full_report_match":true,"snapshot_match":true,
            "certificate_sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}})
    }
    #[test] fn exact_matrices_are_unimodular() {
        for a in [M16, M13] {assert_eq!(multiply(a,invert(a).unwrap()).unwrap(),I);}
    }
    #[test] fn all_four_retained_sources_pass() {
        let r=verify_certificate(&fixture()).unwrap();
        assert_eq!(r["retained_source_presentations"],4);
        assert_eq!(r["passive_slope_transport_verified"],8);
    }
    #[test] fn duplicate_sources_fail() {
        let mut v=fixture();v["sources"][1]=v["sources"][0].clone();
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn signature_substitution_fails() {
        let mut v=fixture();v["sources"][0]["isometry_signature"]=json!("other");
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn changed_passive_slope_fails() {
        let mut v=fixture();v["sources"][0]["controls"]["M16"]["passive_slope"]=json!([0,1]);
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn changed_active_slope_fails() {
        let mut v=fixture();v["sources"][0]["controls"]["M13"]["active_slope"]=json!([0,1]);
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn changed_winding_matrix_fails() {
        let mut v=fixture();v["sources"][0]["controls"]["M16"]["matrix"]=json!(M13);
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn changed_measured_status_fails() {
        let mut v=fixture();v["sources"][0]["controls"]["M16"]["passive_status"]=json!("UNKNOWN");
        assert!(verify_certificate(&v).is_err());
    }
    #[test] fn source_recovery_is_exact() {
        let v=fixture();let id="L7a6:cusp1:slope0,1";
        assert_eq!(recover_source(&v,id).unwrap(),source(&v,id).unwrap().clone());
        assert!(recover_source(&v,"unknown").is_err());
    }
    #[test] fn operator_order_changes_transport() {
        let v=fixture();let id="L7a5:cusp0:slope0,1";
        let ab=transport_word(&v,id,"M16,M13").unwrap();
        let ba=transport_word(&v,id,"M13,M16").unwrap();
        assert_ne!(ab["active_slope_on_original_framing"], ba["active_slope_on_original_framing"]);
        assert_eq!(ab["passive_roundtrip_recovers_original"],true);
    }
    #[test] fn rejects_unknown_operator_and_long_word() {
        let v=fixture();let id="L7a5:cusp0:slope0,1";
        assert!(transport_word(&v,id,"INVENTED").is_err());
        assert!(transport_word(&v,id,"M16,M16,M16,M16,M16,M16,M16").is_err());
    }
    #[test] fn tampered_resident_binding_fails() {
        let mut v=fixture();v["resident"]["snapshot_match"]=json!(false);
        assert!(verify_certificate(&v).is_err());
    }
}
