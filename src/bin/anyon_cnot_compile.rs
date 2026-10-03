//! Native local CNOT calibration; never supplies a phase readout or factors.
use g_momonados::anyon_pair::{CnotBraid, CnotCompileError, FibonacciPair};
use num_bigint::BigUint;
use std::process::ExitCode;

fn report(braid: &CnotBraid, algebra: &FibonacciPair) {
    println!("word={:?}", braid.word);
    println!("scale={}", algebra.format().scale());
    println!("computational={}", braid.residual.computational);
    println!("leakage={}", braid.residual.leakage);
    println!("unitarity={}", braid.residual.unitarity);
}

fn run() -> Result<bool, String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: anyon_cnot_compile N depth beam_width accuracy_bits".into());
    }
    let n = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid source N")?;
    let depth: usize = args[1].parse().map_err(|_| "invalid depth")?;
    let beam: usize = args[2].parse().map_err(|_| "invalid beam width")?;
    let bits: usize = args[3].parse().map_err(|_| "invalid accuracy width")?;
    let algebra = FibonacciPair::new(&n)?;
    if bits >= algebra.format().w_bits as usize {
        return Err("requested accuracy exceeds local fixed-point precision".into());
    }
    let tolerance = algebra.format().scale().to_biguint().unwrap() >> bits;
    println!(
        "source_bits={}, depth={depth}, beam={beam}, accuracy_bits={bits}",
        n.bits()
    );
    match algebra.compile_cnot(depth, beam, &tolerance) {
        Ok(braid) => {
            println!("status=local_residual_accepted");
            report(&braid, &algebra);
            Ok(true)
        }
        Err(CnotCompileError::Budget { best, depth }) => {
            println!("status=budget_exhausted, searched_depth={depth}");
            report(&best, &algebra);
            Ok(false)
        }
        Err(CnotCompileError::Configuration(message)) => Err(message.into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
