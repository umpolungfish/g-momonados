//! Execute a selected Fibonacci fusion weave and close it through its physical inverse.
use g_momonados::{anyon_pair::{FibonacciPair, PairMatrix}, anyon_weave::{compile_weave, FusionStep}};
use num_bigint::{BigInt, BigUint};
use num_traits::Signed;

fn run() -> Result<(), String> {
    vox_core::ordering::cli::intercept();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 { return Err("usage: anyon_weave_probe N-only-decimal".into()); }
    let source = BigUint::parse_bytes(args[0].as_bytes(), 10).ok_or("invalid N-only decimal")?;
    if source.bits() < 200 { return Err("weave probe requires at least 200 source bits".into()); }
    let steps = [FusionStep::F, FusionStep::R(-1), FusionStep::F,
        FusionStep::R(1), FusionStep::F, FusionStep::R(-1)];
    let selected = vox_core::ordering::selected("weave", steps.len())?;
    let weave = compile_weave(&steps)?;
    let algebra = FibonacciPair::new(&source)?;
    let physical = algebra.evaluate(&weave.word)?;
    let inverse_word: Vec<i32> = weave.word.iter().rev().map(|&generator| -generator).collect();
    let inverse = algebra.evaluate(&inverse_word)?;
    let returned = inverse.multiply(&physical, algebra.format());
    let identity = PairMatrix::identity(algebra.format());
    let residual = returned.0.iter().zip(&identity.0).map(|(actual, expected)|
        (&actual.re - &expected.re).abs().max((&actual.im - &expected.im).abs()))
        .max().unwrap_or_else(BigInt::default);
    let tolerance = BigInt::from(4096u16);
    if residual > tolerance {
        return Err(format!("selected weave inverse return exceeded tolerance: residual={residual} tolerance={tolerance}"));
    }
    println!("source_bits={} order={} inverse_order={} fusion_steps={} final_vertex={} crossings={} phase_fifths={} inverse_return_integer_units={} tolerance_integer_units={} return_verified=true",
        source.bits(), selected.label(), selected.inverse().label(), steps.len(), weave.final_vertex,
        weave.word.len(), weave.phase_fifths, residual, tolerance);
    println!("physical_word={}", weave.word.iter().map(i32::to_string).collect::<Vec<_>>().join(" "));
    println!("inverse_word={}", inverse_word.iter().map(i32::to_string).collect::<Vec<_>>().join(" "));
    Ok(())
}

fn main() {
    if let Err(error) = run() { eprintln!("anyon_weave_probe: {error}"); std::process::exit(2); }
}
