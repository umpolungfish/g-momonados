//! Retained scheduling probe, separate from the shared-resident synthesis.
use g_momonados::factor_synthesis::RetainedArmProbe;
use num_bigint::BigUint;
use std::time::Instant;

fn main() -> Result<(), String> {
    let n = BigUint::parse_bytes(include_str!(
        "../../measurements/factor_synthesis_20261011/n.txt").trim().as_bytes(), 10)
        .ok_or("invalid baked N")?;
    let mut resident = RetainedArmProbe::new(n.clone())?;
    let started = Instant::now();
    let mut last_report = Instant::now();
    let mut native_seconds = 0.0;
    let mut quantum_seconds = 0.0;
    let mut turns = 0usize;
    eprintln!("synthesis start source_bits={}", n.bits());
    loop {
        turns += 1;
        let native_started = Instant::now();
        let native = resident.advance_native()?;
        let native_duration = native_started.elapsed();
        native_seconds += native_duration.as_secs_f64();
        let result = match native {
            Some(result) => Some(result),
            None => {
                let quantum_started = Instant::now();
                let mut result = resident.advance_joint()?;
                while result.is_none() && quantum_started.elapsed() < native_duration {
                    result = resident.advance_joint()?;
                }
                quantum_seconds += quantum_started.elapsed().as_secs_f64();
                result
            }
        };
        if last_report.elapsed().as_secs() >= 5 {
            eprintln!("synthesis seconds={:.3} turns={turns} native_seconds={native_seconds:.6} quantum_seconds={quantum_seconds:.6} joint_progress={:?}", started.elapsed().as_secs_f64(), resident.joint_progress());
            last_report = Instant::now();
        }
        if let Some(result) = result {
            let product: BigUint = result.factors.iter().product();
            if product != n { return Err("emitted product differs from N".into()) }
            for factor in result.factors { println!("factor {factor}"); }
            println!("producer {} seconds {:.6}", result.producer, started.elapsed().as_secs_f64());
            return Ok(());
        }
    }
}
