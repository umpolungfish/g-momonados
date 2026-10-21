//! The public conventional-translation boundary assesses N alone.
use num_bigint::BigUint;
use num_traits::{One,Zero};
use serde_json::Value;

pub const HELP:&str="conventional <srfp|dnn|both|read|prepare> <N> [--receipt <path>] [--write-source <path>]\nThe assessed input is N alone. Supplied legs and paired-source files are not accepted.\n";

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct BinaryTree {pub positions:Vec<usize>}
impl BinaryTree {
    pub fn from_value(value:&BigUint)->Self {
        let positions=value.to_bytes_le().iter().enumerate().flat_map(|(i,&byte)|
            (0..8).filter_map(move |bit|if byte&(1<<bit)!=0{Some(8*i+bit)}else{None})).collect();
        Self{positions}
    }
    pub fn evaluate(&self)->BigUint {
        self.positions.iter().fold(BigUint::zero(),|a,&i|a+(BigUint::one()<<i))
    }
}
fn natural(text:&str)->Result<BigUint,String> {
    if text.is_empty()||!text.bytes().all(|b|b.is_ascii_digit()) {
        return Err("N must be a natural numeral".into());
    }
    text.parse().map_err(|_|"invalid natural numeral".into())
}
pub fn command(args:&[&str])->Result<Value,String> {
    let kind=args.first().copied().ok_or(HELP)?;
    if !matches!(kind,"srfp"|"dnn"|"both"|"read"|"prepare") || args.len()!=2 {
        return Err(HELP.into());
    }
    let kind=if matches!(kind,"read"|"prepare"){"both"}else{kind};
    crate::conventional_spectrum::execute(kind,natural(args[1])?)
}
/// CLI and REPL share this boundary. Output paths cannot supply arithmetic data.
pub fn run(args:&[&str])->Result<String,String> {
    if args.is_empty()||args.iter().any(|a|matches!(*a,"help"|"--help"|"-h")) {
        return Ok(HELP.into());
    }
    let mut core=Vec::new();let(mut receipt,mut source)=(None,None);let mut i=0;
    while i<args.len(){match args[i] {
        "--receipt"|"--write-source" if i+1<args.len()=>{
            let target=if args[i]=="--receipt"{&mut receipt}else{&mut source};
            if target.replace(args[i+1]).is_some(){return Err("duplicate output path option".into());}
            i+=2;
        }
        a=>{core.push(a);i+=1;}
    }}
    let report=command(&core)?;
    let rendered=serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?+"\n";
    if let Some(path)=receipt {std::fs::write(path,&rendered).map_err(|e|format!("cannot write receipt: {e}"))?;}
    if let Some(path)=source {
        let text=serde_json::to_string_pretty(&report["source"]).map_err(|e|e.to_string())?+"\n";
        std::fs::write(path,text).map_err(|e|format!("cannot write N-only source: {e}"))?;
    }
    Ok(rendered)
}
