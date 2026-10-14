//! Trigger the self-contained resident membrane constructed by Cargo.
#[cfg(not(target_feature = "crt-static"))]
compile_error!("resident_prepared requires static linkage; use cargo resident-static");
#[path = "../src/bin/ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;
use g_momonados::{resident_glut::{ResidentPhaseCarrier,ResidentClosure},word_tape::WordTape};
use std::time::Instant;
include!(concat!(env!("OUT_DIR"),"/resident_prepared.rs"));

fn main() -> Result<(),String> {
    if PREPARED_RESIDENT.is_empty() { return Err("compile with RESIDENT_SOURCE_FILE pointing to the N-only IMASM source".into()); }
    let loading = Instant::now();
    let record: serde_json::Value = serde_json::from_str(PREPARED_RESIDENT).map_err(|e|e.to_string())?;
    let source = WordTape::from_canonical_word(record["source_word"].as_str().ok_or("baked construction lost its source")?)
        .ok_or("invalid baked source word")?;
    let work = support::work::load_prepared(&record["program"])?;
    let mut carrier = ResidentPhaseCarrier::from_prepared_record(&record,work)?;
    eprintln!("prepared-before-compile nanoseconds={}",record["preparation_nanoseconds"].as_u64()
        .ok_or("baked preparation clock is missing")?);
    eprintln!("baked-state-loaded seconds={:.9}",loading.elapsed().as_secs_f64());
    let triggered = Instant::now();
    let mut last_report = Instant::now();
    while !carrier.advance()? {
        if last_report.elapsed().as_secs() >= 5 {
            eprintln!("resident-trigger elapsed={:.3} product_rows={:?} phase={:?}",
                triggered.elapsed().as_secs_f64(),carrier.product_progress(),carrier.execution_progress());
            eprintln!("resident-Y-clock contractions_calls_ticks_forward_reverse={:?}",carrier.reentry_operations());
            last_report = Instant::now();
        }
    }
    println!("resident-Y-clock contractions_calls_ticks_forward_reverse={:?}",carrier.reentry_operations());
    let returned = carrier.fire()?;
    println!("trigger_to_readout_seconds {:.9}",triggered.elapsed().as_secs_f64());
    println!("phase-numerator {} phase-denominator {}",returned.phase.numerator,returned.phase.denominator);
    println!("phase-monadic-binds {}",returned.phase.monadic_binds);
    eprintln!("resident-return-stage winding-and-boundary");
    let result = match returned.close()? {
        ResidentClosure::Closed(result) => result,
        ResidentClosure::Open(held) => {
            eprintln!("resident-closure remains open phase={}/{}",held.phase.numerator,held.phase.denominator);
            return Err("measured phase did not yet close the held membrane".into());
        }
    };
    let p = WordTape::from_canonical_word(&support::work::word_for_record(&result.divisor)).ok_or("invalid factor return word")?;
    let q = WordTape::from_canonical_word(&support::work::word_for_record(&result.quotient)).ok_or("invalid quotient return word")?;
    if p.mul(&q)!=source { return Err("prepared boundary product did not close its source word".into()); }
    println!("trigger_to_verified_divisor_seconds {:.9}",triggered.elapsed().as_secs_f64());
    println!("factor {}\nquotient {}\nproducer {}",result.divisor,result.quotient,result.producer);
    println!("closed-resident-Y-clock contractions_calls_ticks_forward_reverse={:?}",result.reentry_operations());
    println!("closed-resident-banks {}",result.banked_supports());
    if result.source_word()!=Some(source.as_word()) { return Err("closed membrane lost its original source word".into()); }
    println!("factor-word {}\nquotient-word {}",p.as_word(),q.as_word());
    println!("boundary gamma_delta_closed={} product_closed={} word={}",result.gamma_delta_closed,result.product_closed,result.boundary_word);
    println!("SIC return residual={} tolerance={} signed_residuals={:?}",result.sic.reconstruction.maximum_residual,
        result.sic.reconstruction.tolerance,result.sic.signed_residuals);
    Ok(())
}
