//! Stage 7: in-process resident square-winding capture and independent arithmetic audit.
//! This is a source-bound DecisionArena computation, not a simulated M16 tape.
//! The resident report is retained in full; the SIC label is an independent
//! *source identity* coordinate and is NOT advertised as the resident POVM readout.
use crate::source_braid_state;
use crate::word_tape::WordTape;
use crate::whorl_sic_native;
use num_bigint::BigUint;
use num_traits::One;
use serde_json::{json,Value};
use std::fs;

const SCHEMA:&str="prime-whorl-resident-square-v1";
fn read_decimal(s:&str)->Result<BigUint,String>{
    if s.is_empty() || !s.bytes().all(|b|b.is_ascii_digit()) ||
       (s.len()>1 && s.starts_with('0')) {return Err("noncanonical unsigned decimal".into());}
    BigUint::parse_bytes(s.as_bytes(),10).ok_or_else(||"invalid decimal".into())
}
fn get_decimal<'a>(v:&'a Value,k:&str)->Result<BigUint,String>{
    read_decimal(v.get(k).and_then(Value::as_str).ok_or_else(||format!("missing {k}"))?)
}
fn expected_word(n:&BigUint)->String{
    let mut s=String::from("⊢");
    for i in 0..n.bits(){s.push_str("≻⋈∈");s.push(if n.bit(i){'⊥'}else{'⊤'});s.push('∋');}
    s.push_str("⊙⊡⊣");s
}
/// Audits the *reported* numbers. Does not independently re-run the DAG or
/// authenticate the process that produced a JSON document.
pub fn verify_capture(doc:&Value)->Result<Value,String>{
    if doc.get("schema").and_then(Value::as_str)!=Some(SCHEMA){return Err("invalid resident bridge version".into());}
    let source=doc.get("source").ok_or("missing source")?;
    let n=get_decimal(source,"N")?;
    if n<=BigUint::one(){return Err("N must exceed one".into());}
    let word=source.get("word").and_then(Value::as_str).ok_or("missing canonical word")?;
    if word!=expected_word(&n){return Err("source word does not encode N".into());}
    let native=WordTape::from_biguint(&n);
    if native.as_word()!=word{return Err("WordTape source mismatch".into());}
    let source_lanes=source.get("braid_lanes").and_then(Value::as_array).ok_or("missing braid lanes")?;
    if source_lanes.len()!=2{return Err("expected exactly two ordered lanes".into());}
    let gamma=read_decimal(source_lanes[0].as_str().ok_or("invalid Γ")?)?;
    let lambda=read_decimal(source_lanes[1].as_str().ok_or("invalid Λ")?)?;
    let banks=native.deinterlace();
    if banks.0.to_biguint()!=gamma || banks.1.to_biguint()!=lambda ||
       banks.0.interlace(&banks.1)!=native {
        return Err("braid lane order or reconstruction mismatch".into());
    }
    // This check reuses Stage 6's exact source-SIC tensor validation.
    let sic=source.get("sic_labels").ok_or("missing source SIC labels")?;
    whorl_sic_native::verify_source_snapshot(&n,sic)?;
    let report=doc.get("native_report").ok_or("missing native resident report")?;
    let actual=get_decimal(report,"N")?;
    if n!=actual{return Err("native report source mismatch".into());}
    let report_schema=report.get("schema").and_then(Value::as_str).ok_or("missing resident schema")?;
    let held=match report_schema{
        "retained-square-winding-held/v1"=>true,
        "retained-square-winding/v2"=>false,
        _=>return Err("unsupported source-braid-state schema".into())
    };
    if held {
        if report.get("status").and_then(Value::as_str)!=Some("held") ||
           report.get("factors").is_some_and(|v|!v.is_null()) ||
           report.get("collision_count").is_some_and(|v|!v.is_null()) {
            return Err("held state cannot claim factor or collision count".into());
        }
        if report.get("source_word").and_then(Value::as_str)!=Some(word) {
            return Err("held report source word mismatch".into());
        }
        if report.get("continuation").and_then(Value::as_str).is_none() {
            return Err("held report missing continuation".into());
        }
        return Ok(json!({"valid_arithmetic":true,"status":"held","N":n.to_string(),
            "factor_verified":false,"claims":"source identity, continuation status; no result/factor"}));
    }
    if report.get("source_mass").and_then(Value::as_str)!=Some(n.to_string().as_str()){
        return Err("native source mass does not equal N".into());
    }
    let count=get_decimal(report,"collision_count")?;
    let factors=report.get("factors").ok_or("missing factors field")?;
    let mut factor_verified=false;
    if !factors.is_null(){
        let p=get_decimal(factors,"p")?;let q=get_decimal(factors,"q")?;
        if p<=BigUint::one()||q<=BigUint::one()|| &p*&q!=n {
            return Err("factor pair is not a proper exact divisor pair".into());
        }
        if factors.get("p_word").and_then(Value::as_str)!=Some(expected_word(&p).as_str()) ||
           factors.get("q_word").and_then(Value::as_str)!=Some(expected_word(&q).as_str()) ||
           factors.get("exact_word_product").and_then(Value::as_bool)!=Some(true){
            return Err("factor word proof fields invalid".into());
        }
        let top=&n*4u8+BigUint::one();let two_sum=(&p+&q)*2u8;
        if top<two_sum || count!=top-two_sum {
            return Err("collision count inconsistent with factor root sum".into());
        }
        factor_verified=true;
    }
    Ok(json!({"valid_arithmetic":true,"status":"complete","N":n.to_string(),
        "factor_verified":factor_verified,"collision_count":count.to_string(),
        "claims":"arithmetic consistency only; resident DAG not replayed or authenticated"}))
}
/// Executes the actual source-braid-state DAG in the caller's G-mOMonadOS crate.
pub fn capture(n:&BigUint,budget:usize)->Result<Value,String>{
    if n<=&BigUint::one(){return Err("N must exceed one".into());}
    if budget==0 {return Err("node budget must be positive".into());}
    let source=WordTape::from_biguint(n);
    let (gamma,lambda)=source.deinterlace();
    let sic=whorl_sic_native::source_snapshot(n)?;
    let result=source_braid_state::execute_with_node_budget(source.clone(),budget)?;
    let doc=json!({"schema":SCHEMA,
        "producer":"source_braid_state::execute_with_node_budget",
        "source":{"N":n.to_string(),"word":source.as_word(),
          "braid_lanes":[gamma.to_biguint().to_string(),lambda.to_biguint().to_string()],
          "sic_labels":sic},
        "native_report":result,
        "evidence_scope":"the native_report was emitted by in-process DecisionArena; SIC labels are source-identity labels, not measured POVM probabilities"});
    verify_capture(&doc)?;Ok(doc)
}
fn load(path:&str)->Result<Value,String>{
    let b=fs::read_to_string(path).map_err(|e|e.to_string())?;
    serde_json::from_str(&b).map_err(|e|e.to_string())
}
fn emit(doc:&Value,path:Option<&str>)->Result<(),String>{
    let value=serde_json::to_string_pretty(doc).map_err(|e|e.to_string())?;
    if let Some(p)=path{fs::write(p,format!("{value}\n")).map_err(|e|e.to_string())}
    else {println!("{value}");Ok(())}
}
/// `whorl-resident capture N [--budget N] [--out PATH]` / `inspect FILE` / `verify FILE`.
pub fn command_line(args:&[String])->Result<(),String>{
    if args.len()<2{return Err("usage: whorl-resident capture N [--budget 500000] [--out FILE] | verify FILE | inspect FILE".into());}
    let value=|key:&str|->Result<Option<String>,String>{
        if let Some(i)=args.iter().position(|a|a==key){
            Ok(Some(args.get(i+1).ok_or_else(||format!("{key} requires a value"))?.clone()))
        }else{Ok(None)}
    };
    let out=value("--out")?;
    match args[0].as_str(){
        "capture"=>{
            let n=read_decimal(&args[1])?;
            let budget=value("--budget")?.unwrap_or_else(||"500000".to_string())
                .parse::<usize>().map_err(|_|"invalid budget".to_string())?;
            emit(&capture(&n,budget)?,out.as_deref())
        },
        "replay"=>{
            let report=load(&args[1])?;
            emit(&crate::whorl_resident_replay::verify_snapshot(&report)?,out.as_deref())
        },
        "inspect"|"verify"=>{
            let report=load(&args[1])?;
            let status=verify_capture(&report)?;
            let answer=if args[0]=="inspect"{
                json!({"verification":status,"producer":report["producer"],
                    "source":report["source"],
                    "native_schema":report["native_report"]["schema"],
                    "factor_evidence":report["native_report"]["factors"],
                    "retained_present":report["native_report"].get("retained").is_some()})
            }else{status};
            emit(&answer,out.as_deref())
        },
        _=>Err("unknown command (capture, inspect, verify)".into())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn source_143_is_not_a_factor_pair(){
        let n=BigUint::from(143u64);
        let source=WordTape::from_biguint(&n);
        let (g,l)=source.deinterlace();
        assert_eq!((g.to_biguint(),l.to_biguint()),
            (BigUint::from(3u8),BigUint::from(11u8)));
        assert_ne!(g.to_biguint()*l.to_biguint(),n);
    }
    #[test] fn source_snapshot_roundtrip(){
        let n=BigUint::from(143u64);
        let s=whorl_sic_native::source_snapshot(&n).unwrap();
        whorl_sic_native::verify_source_snapshot(&n,&s).unwrap();
    }
    #[test] fn reject_fabricated_complete_count(){
        let n=BigUint::from(143u64);let source=WordTape::from_biguint(&n);
        let(g,l)=source.deinterlace();
        let bad=json!({"schema":SCHEMA,"source":{"N":"143","word":source.as_word(),
            "braid_lanes":[g.to_biguint().to_string(),l.to_biguint().to_string()],
            "sic_labels":whorl_sic_native::source_snapshot(&n).unwrap()},
            "native_report":{"schema":"retained-square-winding/v2","N":"143",
              "source_mass":"143","collision_count":"526","factors":{"p":"11","q":"13",
              "p_word":expected_word(&BigUint::from(11u8)),
              "q_word":expected_word(&BigUint::from(13u8)),"exact_word_product":true}}});
        assert!(verify_capture(&bad).is_err());
    }
}
