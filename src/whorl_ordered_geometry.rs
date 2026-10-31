//! Stage 11: native audit of fresh Sage/SnapPy ordered surgery observations.
//!
//! Companion to Stage 10's `whorl_geometric_transport`. Geometry is reexecuted
//! by `ordered_geometry.py`; the Rust module independently verifies exact
//! word matrix/slope algebra, linked source identities, and consistency of
//! reported classifications and verified isometry signatures.
use serde_json::{json, Value};
use std::collections::HashSet;

type Mat = [[i128; 2]; 2];
type Slope = [i128; 2];
const I: Mat = [[1, 0], [0, 1]];
const M16: Mat = [[-47, 256], [-9, 49]];
const M13: Mat = [[-38, 169], [-9, 40]];
const W1: &str = "M16,M13";
const W2: &str = "M13,M16";
const SCHEMA: &str = "prime-whorl-ordered-geometry-v1";

fn mat(name: &str) -> Result<Mat, String> {
    match name {"M16" => Ok(M16), "M13" => Ok(M13), _ => Err(format!("invalid winding operation {name}"))}
}
fn mul(a: Mat,b: Mat) -> Result<Mat,String> {
    let mut result = [[0i128;2];2];
    for i in 0..2 {for j in 0..2 {
        let x=a[i][0].checked_mul(b[0][j]).ok_or("matrix overflow")?;
        let y=a[i][1].checked_mul(b[1][j]).ok_or("matrix overflow")?;
        result[i][j]=x.checked_add(y).ok_or("matrix overflow")?;
    }}
    Ok(result)
}
fn act(a: Mat, v:Slope) -> Result<Slope,String> {
    let mut o=[0i128;2];
    for i in 0..2 {
        let x=a[i][0].checked_mul(v[0]).ok_or("slope overflow")?;
        let y=a[i][1].checked_mul(v[1]).ok_or("slope overflow")?;
        o[i]=x.checked_add(y).ok_or("slope overflow")?;
    }
    Ok(o)
}
fn invert(a:Mat) -> Result<Mat,String> {
    let a1=a[0][0].checked_mul(a[1][1]).ok_or("determinant overflow")?;
    let b1=a[0][1].checked_mul(a[1][0]).ok_or("determinant overflow")?;
    let det=a1.checked_sub(b1).ok_or("determinant overflow")?;
    if det!=1 && det!= -1 {return Err("non-unimodular word matrix".into());}
    let b=a[0][1].checked_neg().ok_or("inverse overflow")?;
    let c=a[1][0].checked_neg().ok_or("inverse overflow")?;
    Ok([[a[1][1]/det,b/det],[c/det,a[0][0]/det]])
}
fn product(word:&str)->Result<Mat,String> {
    let mut r=I;
    for name in word.split(',') {r=mul(mat(name)?,r)?;}
    Ok(r)
}
fn slope(v:&Value,where_:&str)->Result<Slope,String> {
    let a=v.as_array().ok_or_else(||format!("{where_}: expected slope array"))?;
    if a.len()!=2 {return Err(format!("{where_}: expected slope pair"));}
    Ok([i128::from(a[0].as_i64().ok_or("slope must be an integer")?),
        i128::from(a[1].as_i64().ok_or("slope must be an integer")?)])
}
fn classification(v:&Value,where_:&str)->Result<&'static str,String> {
    let status=v.get("status").and_then(Value::as_str).ok_or_else(||format!("{where_}: missing isometry status"))?;
    match (status,v.get("value")) {
        ("ISOMETRIC",Some(Value::Bool(true)))=>Ok("ISOMETRIC"),
        ("NOT_ISOMETRIC",Some(Value::Bool(false)))=>Ok("NOT_ISOMETRIC"),
        ("INCONCLUSIVE",Some(Value::Null))=>Ok("INCONCLUSIVE"),
        _=>Err(format!("{where_}: invalid isometry status/value pair"))
    }
}
fn checked_measurement(v:&Value,where_:&str,target_sig:&str)->Result<&'static str,String> {
    let status=classification(&v["isometry"],where_)?;
    let sig=v.get("signature").ok_or_else(||format!("{where_}: missing signature record"))?;
    match sig.get("verified").and_then(Value::as_bool) {
        Some(true)=>{
            let val=sig.get("value").and_then(Value::as_str).filter(|s|!s.is_empty())
                .ok_or_else(||format!("{where_}: verified signature missing"))?;
            if status!="INCONCLUSIVE" && (val==target_sig)!=(status=="ISOMETRIC") {
                return Err(format!("{where_}: verified signature contradicts target isometry"));
            }
        },
        Some(false)=>{
            if !sig.get("value").unwrap_or(&Value::Null).is_null() {
                return Err(format!("{where_}: unverified signature has a value"));
            }
        },
        _=>return Err(format!("{where_}: invalid signature verification flag"))
    }
    Ok(status)
}
fn source<'a>(cert:&'a Value,id:&str)->Result<&'a Value,String> {
    cert["sources"].as_array().ok_or("Stage 10 sources missing")?.iter()
        .find(|s|s["source_id"].as_str()==Some(id)).ok_or_else(||format!("unknown source {id}"))
}
fn expected_ids()->HashSet<String>{
    let mut out=HashSet::new();
    for link in ["L7a5","L7a6"] {for cusp in 0..2 {
        out.insert(format!("{link}:cusp{cusp}:slope0,1"));
    }}
    out
}
/// Validate exact active/passive geometric transport and check saved SnapPy
/// observations for consistency. Does not rerun SnapPy or authenticate a hash.
pub fn verify(cert:&Value,report:&Value)->Result<Value,String>{
    crate::whorl_geometric_transport::verify_certificate(cert)?;
    if report["schema"].as_str()!=Some(SCHEMA) || report["status"].as_str()!=Some("complete") ||
       report["geometry_execution"].as_str()!=Some("snappy") {
        return Err("invalid Stage 11 schema/execution status".into());
    }
    if report["stage10_resident_binding"]!=cert["resident"] {
        return Err("Stage 10 resident certificate binding changed".into());
    }
    if report["target"]["manifold"].as_str()!=Some("m137") ||
       report["target"]["stage10_verified_signature"]!=cert["target"]["isometry_signature"] {
        return Err("target manifold or signature changed".into());
    }
    let target_sig=cert["target"]["isometry_signature"].as_str().ok_or("target signature missing")?;
    let fresh=&report["target"]["fresh_signature"];
    let requested=report["verified_signatures_requested"].as_bool().ok_or("signature request flag missing")?;
    if fresh["verified"].as_bool()==Some(true) {
        if fresh["value"].as_str()!=Some(target_sig) {return Err("fresh target signature differs".into());}
    } else if requested || fresh["verified"].as_bool()!=Some(false) || !fresh["value"].is_null() {
        return Err("target signature not verified as requested".into());
    }
    if report["words"]!=json!([W1,W2]) {return Err("expected two ordered words".into());}
    let entries=report["sources"].as_array().ok_or("missing Stage 11 sources")?;
    if entries.len()!=4 {return Err("expected exactly four source rows".into());}
    let mut seen=HashSet::new();
    let mut active=[0u64;3];let mut passive=[0u64;3];let mut compared=[0u64;3];
    for entry in entries {
        let id=entry["source_id"].as_str().ok_or("missing source ID")?;
        if !expected_ids().contains(id) || !seen.insert(id.to_string()) {return Err("duplicate/unexpected source".into());}
        let src=source(cert,id)?;
        if entry.get("retained_source")!=Some(src) {return Err(format!("retained source substituted: {id}"));}
        let s=slope(&src["slope"],"original")?;
        let branches=entry["branches"].as_object().ok_or("missing branches")?;
        if branches.len()!=2 {return Err(format!("{id}: unexpected branches"));}
        let mut signatures=Vec::new();
        for word in [W1,W2] {
            let b=branches.get(word).ok_or_else(||format!("{id}: missing {word}"))?;
            let m=product(word)?;
            let words:Vec<&str>=word.split(',').collect();
            if b["word"]!=json!(words) || b["matrix"]!=json!(m) {return Err(format!("{id}: word matrix mismatch"));}
            if slope(&b["active_slope"],"active")?!=act(m,s)? {
                return Err(format!("{id}: wrong active slope"));
            }
            let p=act(invert(m)?,s)?;
            if slope(&b["passive_coordinates"],"passive")?!=p ||
               b["passive_roundtrip"].as_bool()!=Some(true) || act(m,p)?!=s {
                return Err(format!("{id}: wrong passive slope/roundtrip"));
            }
            let a=checked_measurement(&b["active"],"active",target_sig)?;
            let pstatus=checked_measurement(&b["passive"],"passive",target_sig)?;
            active[index(a)]+=1;passive[index(pstatus)]+=1;
            signatures.push(b["active"]["signature"].clone());
        }
        let pair=&entry["active_order_comparison"];
        let status=classification(pair,"order comparison")?;
        compared[index(status)]+=1;
        if signatures.len()==2 && signatures[0]["verified"]==true && signatures[1]["verified"]==true && status!="INCONCLUSIVE" {
            let same=signatures[0]["value"]==signatures[1]["value"];
            if same != (status=="ISOMETRIC") {return Err(format!("{id}: order comparison contradicts verified signatures"));}
        }
    }
    if seen != expected_ids() {return Err("incomplete source set".into());}
    fn counts(a:[u64;3])->Value {
        json!({"ISOMETRIC":a[0],"NOT_ISOMETRIC":a[1],"INCONCLUSIVE":a[2]})
    }
    let summary=json!({"source_presentations":4,"active":counts(active),"passive":counts(passive),
        "active_order_comparison":counts(compared),
        "all_passive_controls_isometric":passive[0]==8,
        "all_order_comparisons_resolved":compared[2]==0});
    if report["summary"]!=summary {return Err("report summary contradicts rows".into());}
    let hash=report["stage10_certificate_sha256"].as_str().ok_or("missing Stage 10 sha256")?;
    if hash.len()!=64 || !hash.bytes().all(|b|b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("malformed Stage 10 content SHA-256".into());
    }
    Ok(json!({"status":"complete","schema":"prime-whorl-ordered-native-verification-v1",
        "source_presentations":4,"ordered_word_branches_verified":8,
        "active_exact_transports":8,"passive_inverse_transports":8,
        "measurements_schema_consistent":true,"summary":summary,
        "claims":"exact native integer transport and retained source equality; Sage observations are checked for internal consistency, not independently recomputed by Rust; hashes are rechecked by Python verifier"}))
}
fn index(status:&str)->usize {
    match status {"ISOMETRIC"=>0,"NOT_ISOMETRIC"=>1,_=>2}
}
/// Return one retained source plus BOTH measured word branches and comparison.
pub fn recover(cert:&Value,report:&Value,id:&str)->Result<Value,String>{
    verify(cert,report)?;
    report["sources"].as_array().ok_or("missing sources")?.iter()
        .find(|s|s["source_id"].as_str()==Some(id))
        .cloned().ok_or_else(||format!("unknown source: {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cert()->Value {
        let mut sources=Vec::new();
        for link in ["L7a5","L7a6"] {for cusp in 0..2 {
            let mut controls=serde_json::Map::new();
            for (name,m) in [("M16",M16),("M13",M13)] {
                controls.insert(name.into(),json!({"matrix":m,"passive_slope":act(invert(m).unwrap(),[0,1]).unwrap(),
                    "active_slope":act(m,[0,1]).unwrap(),"passive_status":"ISOMETRIC","active_status":"NOT_ISOMETRIC"}));
            }
            sources.push(json!({"source_id":format!("{link}:cusp{cusp}:slope0,1"),"link":link,
                "filled_cusp":cusp,"slope":[0,1],"framing":"SnapPy default peripheral basis",
                "isometry_signature":"SIG", "controls":controls}));
        }}
        let sha = "a".repeat(64);
        json!({"schema":"prime-whorl-geometric-transport-v1",
            "target":{"manifold":"m137","verified_signatures_requested":true,"isometry_signature":"SIG"},
            "sources":sources,"resident":{"source_N":"143","collision_count":"525","factor_verified":true,
            "stage9_status":"complete","full_report_match":true,"snapshot_match":true,
            "certificate_sha256":sha}})
    }
    fn measurement(sig:&str,same:bool)->Value {
        json!({"isometry":{"status":if same {"ISOMETRIC"} else {"NOT_ISOMETRIC"},"value":same,"error":null},
            "volume":2.0,"homology":"Z","solution_type":"all tetrahedra positively oriented",
            "signature":{"verified":true,"value":sig,"error":null}})
    }
    fn fixture()->(Value,Value) {
        let c=cert();let mut sources=Vec::new();
        for s in c["sources"].as_array().unwrap(){
            let mut branches=serde_json::Map::new();
            for word in [W1,W2]{
                let m=product(word).unwrap();
                branches.insert(word.into(),json!({"word":word.split(',').collect::<Vec<_>>(),"matrix":m,
                    "active_slope":act(m,[0,1]).unwrap(),"passive_coordinates":act(invert(m).unwrap(),[0,1]).unwrap(),
                    "passive_roundtrip":true,"active":measurement(if word==W1 {"sigA"}else{"sigB"},false),
                    "passive":measurement("SIG",true)}));
            }
            sources.push(json!({"source_id":s["source_id"],"retained_source":s,
                "branches":branches,"active_order_comparison":{"status":"NOT_ISOMETRIC","value":false,"error":null}}));
        }
        let stage10_sha = "b".repeat(64);
        let r=json!({"schema":SCHEMA,"status":"complete","geometry_execution":"snappy",
            "verified_signatures_requested":true,
            "stage10_certificate_sha256":stage10_sha,"stage10_resident_binding":c["resident"],
            "target":{"manifold":"m137","stage10_verified_signature":"SIG",
                "fresh_signature":{"verified":true,"value":"SIG","error":null}},
            "words":[W1,W2],"sources":sources,
            "summary":{"source_presentations":4,"active":{"ISOMETRIC":0,"NOT_ISOMETRIC":8,"INCONCLUSIVE":0},
              "passive":{"ISOMETRIC":8,"NOT_ISOMETRIC":0,"INCONCLUSIVE":0},
              "active_order_comparison":{"ISOMETRIC":0,"NOT_ISOMETRIC":4,"INCONCLUSIVE":0},
              "all_passive_controls_isometric":true,"all_order_comparisons_resolved":true}});
        (c,r)
    }
    #[test] fn ordered_product_correct(){
        assert_eq!(product(W1).unwrap(),[[265,-1447],[63,-344]]);
        assert_eq!(product(W2).unwrap(),[[-518,2297],[-99,439]]);
    }
    #[test] fn valid_report(){let (c,r)=fixture();assert_eq!(verify(&c,&r).unwrap()["ordered_word_branches_verified"],8);}
    #[test] fn recover_preserves_source(){let (c,r)=fixture();let id="L7a6:cusp1:slope0,1";
        assert_eq!(recover(&c,&r,id).unwrap()["retained_source"],c["sources"][3]);}
    #[test] fn reject_unknown_source(){let(c,r)=fixture();assert!(recover(&c,&r,"missing").is_err());}
    #[test] fn reject_swapped_matrix(){let(c,mut r)=fixture();r["sources"][0]["branches"][W1]["matrix"]=json!(I);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_wrong_active_slope(){let(c,mut r)=fixture();r["sources"][0]["branches"][W1]["active_slope"]=json!([2,2]);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_wrong_passive(){let(c,mut r)=fixture();r["sources"][0]["branches"][W2]["passive_coordinates"]=json!([1,1]);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_modified_source(){let(c,mut r)=fixture();r["sources"][0]["retained_source"]["filled_cusp"]=json!(1);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_fake_passive_classification(){let(c,mut r)=fixture();r["sources"][0]["branches"][W1]["passive"]["isometry"]["status"]=json!("NOT_ISOMETRIC");
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_signature_conflict(){let(c,mut r)=fixture();r["sources"][0]["branches"][W1]["active"]["signature"]["value"]=json!("SIG");
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_comparison_conflict(){let(c,mut r)=fixture();r["sources"][0]["active_order_comparison"]["status"]=json!("ISOMETRIC");
        r["sources"][0]["active_order_comparison"]["value"]=json!(true);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_bad_summary(){let(c,mut r)=fixture();r["summary"]["passive"]["ISOMETRIC"]=json!(7);
        assert!(verify(&c,&r).is_err());}
    #[test] fn reject_replaced_resident_binding(){let(c,mut r)=fixture();r["stage10_resident_binding"]["collision_count"]=json!("0");
        assert!(verify(&c,&r).is_err());}
}
