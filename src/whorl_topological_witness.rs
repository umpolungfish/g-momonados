//! Stage 12: exact native checking of measured topological obstructions.
//!
//! Source observations are produced by Sage/SnapPy in Stage 11.2. This module
//! does not derive Alexander polynomials or cover homology from triangulations.
//! It checks source byte digests, all three report content digests, retained
//! source identity, exact Stage 11 winding, canonical integer invariants and
//! non-homeomorphism implications. No CUDA device is needed for the audit.
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, Zero};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs;

const SCHEMA: &str = "prime-whorl-native-topological-witness-v1";
const WORDS: [&str; 2] = ["M16,M13", "M13,M16"];
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 implemented within the module to avoid modifying the host's Cargo
/// dependency graph. This is an unkeyed content digest, not an origin signature.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                           0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 { padded.push(0); }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(t1);
            d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        for (state, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *state = state.wrapping_add(v);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect::<String>()
}

pub fn file_sha256(path: &str) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    Ok(sha256_hex(&data))
}

fn check_content_hash(v: &Value, label: &str) -> Result<(), String> {
    let expected = v.get("content_sha256").and_then(Value::as_str)
        .ok_or_else(|| format!("{label}: missing content digest"))?;
    let mut body = v.clone();
    body.as_object_mut().ok_or_else(|| format!("{label}: not an object"))?
        .remove("content_sha256");
    let encoded = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
    if sha256_hex(&encoded) != expected { return Err(format!("{label}: content digest mismatch")); }
    Ok(())
}
fn checked_decimal(input: &str, signed: bool) -> Result<BigInt, String> {
    if input.is_empty() || input.starts_with('+') || input.starts_with("-0") ||
        (input.starts_with('0') && input.len() > 1) { return Err("noncanonical decimal".into()); }
    if !input.bytes().enumerate().all(|(i,b)| b.is_ascii_digit() || (signed && i == 0 && b == b'-')) {
        return Err("nondecimal integer".into());
    }
    let val = BigInt::parse_bytes(input.as_bytes(), 10).ok_or("invalid integer")?;
    if !signed && val.is_negative() { return Err("negative homology divisor".into()); }
    if val.to_string() != input { return Err("noncanonical integer representation".into()); }
    Ok(val)
}
fn normalized_poly(nums: &[BigInt]) -> Result<Vec<BigInt>, String> {
    if nums.is_empty() || nums.len() > 100_000 || nums.iter().all(BigInt::is_zero) {
        return Err("empty/zero/overlarge Alexander polynomial".into());
    }
    let first = nums.iter().position(|x| !x.is_zero()).ok_or("zero polynomial")?;
    let last = nums.iter().rposition(|x| !x.is_zero()).ok_or("zero polynomial")?;
    let forward = nums[first..=last].to_vec();
    let reversed = forward.iter().rev().cloned().collect::<Vec<_>>();
    let positive = |xs: Vec<BigInt>| -> Vec<BigInt> {
        if xs[0].is_negative() { xs.iter().map(|x| -x).collect() } else { xs }
    };
    let a = positive(forward);
    let b = positive(reversed);
    Ok(if a <= b { a } else { b })
}
fn string_bigints(v: &Value, signed: bool, what: &str) -> Result<Vec<BigInt>, String> {
    let arr = v.as_array().ok_or_else(|| format!("{what}: expected array"))?;
    if arr.len() > 100_000 { return Err(format!("{what}: too many values")); }
    arr.iter().map(|x| checked_decimal(x.as_str().ok_or("expected decimal string")?,signed)).collect()
}
fn json_bigints(v: &Value, signed: bool) -> Result<Vec<BigInt>, String> {
    let arr = v.as_array().ok_or("raw invariant must be array")?;
    if arr.len() > 100_000 {return Err("raw invariant too long".into());}
    arr.iter().map(|x| {
        if !x.is_number() {return Err("source invariant must be JSON integers".into());}
        let n = x.to_string();
        checked_decimal(&n, signed)
    }).collect()
}
fn invariant_factors(nums: &[BigInt]) -> Result<Vec<String>, String> {
    if nums.len() > 100_000 {return Err("too many group factors".into());}
    if nums.iter().any(BigInt::is_negative) {return Err("negative group factor".into());}
    let zeros = nums.iter().filter(|x| x.is_zero()).count();
    let mut factors = nums.iter().filter(|x| **x > BigInt::from(1u8))
        .map(|x| x.to_biguint().ok_or("negative homology factor"))
        .collect::<Result<Vec<BigUint>,_>>()?;
    factors.sort();
    // Replace each offending cyclic summand pair (a,b) by (gcd(a,b),lcm(a,b)).
    // Trivial factors (1) are discarded. The result is a canonical invariant
    // factor decomposition even when SnapPy emits a different cyclic basis.
    let mut steps = 0usize;
    loop {
        let offending = factors.windows(2).position(|w| (&w[1] % &w[0]) != BigUint::zero());
        let Some(i) = offending else { break; };
        steps += 1;
        if steps > 100_000 {return Err("group normalization iteration limit".into());}
        let a = factors[i].clone(); let b = factors[i+1].clone();
        let mut x = a; let mut y = b;
        while !y.is_zero() {let r = &x % &y; x = y; y = r;}
        let lcm = (&factors[i] / &x) * &factors[i + 1];
        factors[i] = x;
        factors[i+1] = lcm;
        factors.retain(|n| *n > BigUint::from(1u8));
        factors.sort();
    }
    let mut result = vec!["0".to_string(); zeros];
    result.extend(factors.iter().map(ToString::to_string));
    Ok(result)
}
fn source_invariant(probe: &Value, method: &str) -> Result<Value, String> {
    if probe["method"].as_str() != Some(method) ||
        probe["status"].as_str() != Some("EXACT_INVARIANT") ||
        probe["permanent_fill_one_cusp"].as_bool() != Some(true) ||
        probe["first_homology_elementary_divisors"] != json!([0]) {
        return Err("not a completed exact permanently-filled H1=Z probe".into());
    }
    let h1 = invariant_factors(&json_bigints(&probe["first_homology_elementary_divisors"], false)?)?;
    if h1 != vec!["0".to_string()] {return Err("first homology not Z".into());}
    if method == "alexander" {
        let raw = json_bigints(&probe["alexander_coefficients"], true)?;
        let norm = normalized_poly(&raw)?;
        if raw != norm {return Err("noncanonical saved Alexander polynomial".into());}
        return Ok(json!({"kind":"alexander","coefficients":norm.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "first_homology_invariant_factors":h1,"permanent_fill_one_cusp":true}));
    }
    let degree = match method {"cyclic2"=>2,"cyclic3"=>3,_=>return Err("unsupported exact witness method".into())};
    if probe["cover_degree"].as_u64() != Some(degree) {return Err("cyclic-cover degree mismatch".into());}
    let factors = invariant_factors(&json_bigints(&probe["cover_homology_elementary_divisors"],false)?)?;
    Ok(json!({"kind":"cyclic_cover_h1","degree":degree,"invariant_factors":factors,
        "first_homology_invariant_factors":h1,"permanent_fill_one_cusp":true}))
}
fn check_canonical_invariant(inv: &Value, method: &str) -> Result<(), String> {
    if inv["permanent_fill_one_cusp"].as_bool()!=Some(true) ||
        inv["first_homology_invariant_factors"]!=json!(["0"]) {
        return Err("invariant lacks H1=Z one-cusp prerequisite".into());
    }
    if method == "alexander" {
        if inv["kind"].as_str()!=Some("alexander") {return Err("wrong invariant type".into());}
        let nums=string_bigints(&inv["coefficients"],true,"Alexander")?;
        if normalized_poly(&nums)? != nums {return Err("noncanonical normalized Alexander polynomial".into());}
    } else {
        let degree=if method=="cyclic2" {2} else {3};
        if inv["kind"].as_str()!=Some("cyclic_cover_h1") || inv["degree"].as_u64()!=Some(degree) {
            return Err("wrong cyclic-cover type/degree".into());
        }
        let nums=string_bigints(&inv["invariant_factors"],false,"cover H1")?;
        let expected=invariant_factors(&nums)?;
        if expected != inv["invariant_factors"].as_array().ok_or("group factors not array")?
            .iter().map(|x| x.as_str().unwrap_or("").to_string()).collect::<Vec<_>>() {
            return Err("noncanonical cyclic-cover group".into());
        }
    }
    Ok(())
}
fn raw_row<'a>(raw: &'a Value, id: &str) -> Result<&'a Value, String> {
    raw["results"].as_array().ok_or("rescue has no results")?.iter()
        .find(|x| x["source_id"].as_str()==Some(id))
        .ok_or_else(|| format!("rescue missing source {id}"))
}
fn ordered_row<'a>(raw: &'a Value, id: &str) -> Result<&'a Value, String> {
    raw["sources"].as_array().ok_or("ordered report missing sources")?.iter()
        .find(|x| x["source_id"].as_str()==Some(id))
        .ok_or_else(|| format!("ordered report missing source {id}"))
}
fn method_spec(method: &str) -> Result<(&'static str,&'static str),String> {
    match method {
        "distinct_alexander_polynomials" => Ok(("alexander","alexander_difference")),
        "distinct_cyclic2_cover_homology" => Ok(("cyclic2","cyclic_cover_h1_difference")),
        "distinct_cyclic3_cover_homology" => Ok(("cyclic3","cyclic_cover_h1_difference")),
        _ => Err("unsupported/missing exact non-homeomorphism method".into())
    }
}
/// Check the Stage 12 JSON against Stage 10, Stage 11, and original measured
/// Stage 11.2 invariants. This is NOT a fresh SnapPy polynomial computation.
pub fn verify(base: &Value, ordered: &Value, rescue: &Value, witness: &Value,
              input_hashes: &[String; 3]) -> Result<Value, String> {
    crate::whorl_geometric_transport::verify_certificate(base)?;
    crate::whorl_ordered_geometry::verify(base, ordered)?;
    check_content_hash(ordered, "Stage 11")?;
    check_content_hash(rescue, "Stage 11.2")?;
    check_content_hash(witness, "Stage 12")?;
    if witness["schema"].as_str()!=Some(SCHEMA) || witness["status"].as_str()!=Some("complete") ||
       rescue["schema"].as_str()!=Some("prime-whorl-ordered-topology-rescue-v1") ||
       rescue["status"].as_str()!=Some("complete") ||
       rescue["summary"]["all_orders_separated"].as_bool()!=Some(true) {
        return Err("unsupported report schema/status/summary".into());
    }
    for (name, expected) in [("stage10_sha256",&input_hashes[0]),
                             ("stage11_sha256",&input_hashes[1]),
                             ("topology_rescue_sha256",&input_hashes[2])] {
        if witness["inputs"][name].as_str()!=Some(expected.as_str()) {
            return Err(format!("Stage 12 input byte digest mismatch: {name}"));
        }
    }
    if rescue["stage10_sha256"].as_str()!=Some(input_hashes[0].as_str()) ||
       rescue["stage11_sha256"].as_str()!=Some(input_hashes[1].as_str()) ||
       ordered["stage10_certificate_sha256"].as_str()!=Some(input_hashes[0].as_str()) {
        return Err("earlier report source binding mismatch".into());
    }
    if witness["inputs"]["resident_stage9"]!=base["resident"] ||
       witness["target"]["manifold"].as_str()!=Some("m137") ||
       witness["target"]["isometry_signature"]!=base["target"]["isometry_signature"] {
        return Err("resident replay or target signature substitution".into());
    }
    let sources=witness["sources"].as_array().ok_or("missing witnesses")?;
    if sources.len()!=4 || rescue["results"].as_array().ok_or("missing rescue rows")?.len()!=4 {
        return Err("expected exactly four retained source rows".into());
    }
    let mut seen=HashSet::new();
    let mut method_counts=BTreeMap::<String,u64>::new();
    let mut polys=0u64;let mut covers=0u64;
    for src in sources {
        let id=src["source_id"].as_str().ok_or("missing source ID")?;
        if !seen.insert(id) {return Err("duplicate source witness".into());}
        let prior=ordered_row(ordered,id)?;
        let saved=raw_row(rescue,id)?;
        if src["retained_source"]!=prior["retained_source"] ||
           src["retained_source"]["source_id"].as_str()!=Some(id) {
            return Err(format!("{id}: retained source mismatch"));
        }
        if saved["comparison"]["status"].as_str()!=Some("NOT_ISOMETRIC") {
            return Err(format!("{id}: source topological distinction not recorded"));
        }
        let method=saved["comparison"]["method"].as_str().ok_or("missing measured method")?;
        let (probe_name,kind)=method_spec(method)?;
        let degree=if probe_name=="cyclic2" {2} else {3};
        let proof=&src["witness"];
        if proof["method"].as_str()!=Some(method) || proof["kind"].as_str()!=Some(kind) ||
            proof["status"].as_str()!=Some("NOT_HOMEOMORPHIC") ||
            (kind=="cyclic_cover_h1_difference" && proof["degree"].as_u64()!=Some(degree)) {
            return Err(format!("{id}: invalid explicit topological witness"));
        }
        let branches=src["branches"].as_object().ok_or("missing word branches")?;
        if branches.len()!=2 {return Err("expected exactly two ordered branches".into());}
        let mut values=Vec::new();
        for word in WORDS {
            let b=branches.get(word).ok_or("missing ordered word")?;
            let old=&prior["branches"][word];
            if b["word"]!=old["word"] || b["matrix"]!=old["matrix"] ||
               b["active_slope"]!=old["active_slope"] ||
               b["active_slope"]!=saved["branches"][word]["slope"] {
                return Err(format!("{id}: winding slope or matrix mismatch"));
            }
            let saved_probe=&saved["branches"][word]["probes"][probe_name];
            let expected=source_invariant(saved_probe,probe_name)?;
            let inv=&b["invariant"];
            check_canonical_invariant(inv,probe_name)?;
            if *inv != expected {return Err(format!("{id}: invariant altered or not sourced from Sage result"));}
            values.push(if probe_name=="alexander" {inv["coefficients"].clone()} else {inv["invariant_factors"].clone()});
        }
        if values[0]==values[1] {
            return Err(format!("{id}: no difference between exact normalized invariants"));
        }
        *method_counts.entry(method.to_string()).or_insert(0)+=1;
        if probe_name=="alexander" {polys+=1;} else {covers+=1;}
    }
    let mut counted=serde_json::Map::new();
    for (k,v) in &method_counts {counted.insert(k.clone(),json!(v));}
    let summary=json!({"sources":4,"not_homeomorphic":4,"classification_methods":counted});
    if witness["summary"]!=summary || seen.len()!=4 ||
       rescue["summary"]["active_order_comparison"]["NOT_ISOMETRIC"].as_u64()!=Some(4) ||
       rescue["summary"]["active_order_comparison"]["INCONCLUSIVE"].as_u64()!=Some(0) ||
       rescue["summary"]["classification_methods"]!=summary["classification_methods"] {
        return Err("witness summary differs from measured comparison evidence".into());
    }
    Ok(json!({"schema":"prime-whorl-native-topological-audit-v1","status":"complete",
        "retained_sources":4,"ordered_branches":8,"exact_obstruction_witnesses":4,
        "alexander_witnesses":polys,"cyclic_cover_h1_witnesses":covers,
        "all_report_digests_valid":true,"all_input_byte_digests_valid":true,
        "source_recovery_available":true,"stage9_binding_preserved":true,
        "geometric_scope":"native arithmetic, polynomial/group normalization and source-derived topological obstruction checking; original invariants produced by Sage/SnapPy, not recalculated from triangulations by Rust"}))
}
/// Recover the exact retained source, two ordered active slopes, and both
/// imported algebraic invariants after verifying the entire chain.
pub fn recover(base:&Value,ordered:&Value,rescue:&Value,witness:&Value,
               hashes:&[String;3],id:&str)->Result<Value,String>{
    verify(base,ordered,rescue,witness,hashes)?;
    witness["sources"].as_array().ok_or("missing sources")?.iter()
        .find(|src|src["source_id"].as_str()==Some(id)).cloned()
        .ok_or_else(||format!("unknown retained source: {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn sha256_known_vectors() {
        assert_eq!(sha256_hex(b""),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
    #[test] fn polynomial_reverse_is_unit() {
        let a=vec![1.into(),(-2).into(),3.into()];
        let b=vec![3.into(),(-2).into(),1.into()];
        assert_eq!(normalized_poly(&a).unwrap(),normalized_poly(&b).unwrap());
    }
    #[test] fn polynomial_sign_unit() {
        let a=vec![2.into(),(-3).into(),2.into()];
        let b=vec![(-2).into(),3.into(),(-2).into()];
        assert_eq!(normalized_poly(&a).unwrap(),normalized_poly(&b).unwrap());
    }
    #[test] fn integer_content_not_normalized_away() {
        assert_ne!(normalized_poly(&[2.into(),(-4).into()]).unwrap(),
                   normalized_poly(&[1.into(),(-2).into()]).unwrap());
    }
    #[test] fn invalid_polynomial_rejected() {
        assert!(normalized_poly(&[]).is_err());
        assert!(normalized_poly(&[BigInt::zero()]).is_err());
    }
    #[test] fn equivalent_cyclic_groups_not_separated() {
        let x=invariant_factors(&[2.into(),3.into(),0.into()]).unwrap();
        let y=invariant_factors(&[6.into(),0.into()]).unwrap();
        assert_eq!(x,y);
    }
    #[test] fn distinct_cyclic_groups_separated() {
        assert_ne!(invariant_factors(&[2.into(),4.into(),0.into()]).unwrap(),
                   invariant_factors(&[2.into(),8.into(),0.into()]).unwrap());
    }
    #[test] fn reject_negative_group() {
        assert!(invariant_factors(&[BigInt::from(-2)]).is_err());
    }
    #[test] fn reject_noncanonical_decimals() {
        assert!(checked_decimal("02",true).is_err());
        assert!(checked_decimal("-0",true).is_err());
        assert!(checked_decimal("+2",true).is_err());
    }
    #[test] fn check_alexander_data() {
        let raw=json!({"method":"alexander","status":"EXACT_INVARIANT",
            "permanent_fill_one_cusp":true,"first_homology_elementary_divisors":[0],
            "alexander_coefficients":[1,-4,1]});
        let value=source_invariant(&raw,"alexander").unwrap();
        assert_eq!(value["coefficients"],json!(["1","-4","1"]));
        check_canonical_invariant(&value,"alexander").unwrap();
    }
    #[test] fn check_cover_data() {
        let raw=json!({"method":"cyclic2","status":"EXACT_INVARIANT",
            "permanent_fill_one_cusp":true,"first_homology_elementary_divisors":[0],
            "cover_degree":2,"cover_homology_elementary_divisors":[0,2,3]});
        let value=source_invariant(&raw,"cyclic2").unwrap();
        assert_eq!(value["invariant_factors"],json!(["0","6"]));
        check_canonical_invariant(&value,"cyclic2").unwrap();
    }
    #[test] fn invalid_source_probe_status() {
        let raw=json!({"method":"alexander","status":"INCONCLUSIVE",
            "permanent_fill_one_cusp":true,"first_homology_elementary_divisors":[0],
            "alexander_coefficients":[1,2]});
        assert!(source_invariant(&raw,"alexander").is_err());
    }
    #[test] fn canonical_json_digest() {
        let value=json!({"b":2,"a":["v",3]});
        assert_eq!(sha256_hex(&serde_json::to_vec(&value).unwrap()),
            "728f0f5561bac455fbc6b19bee549f0b0b67f955badf4bc2b16da694958cf3e1");
    }
}
