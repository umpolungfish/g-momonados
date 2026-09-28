//! Baked ABC cutoff membrane. Usage: membrane_one.sh abc numerator denominator cutoff...
extern crate alloc;
#[path = "../abc_iutt.rs"] mod abc_iutt;
#[path = "../baked_membrane.rs"] mod baked_membrane;

fn run() -> Result<(), String> {
    let values = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
    if values.len() < 3 || values[1] == 0 {
        return Err("ABC needs epsilon numerator, nonzero denominator, and cutoffs".into());
    }
    // Bound the hosted scan's table and its u64 radical products explicitly.
    if values[2..].iter().any(|&c| c > 1_000_000) {
        return Err("ABC cutoff exceeds the hosted scan range 0..=1000000".into());
    }
    let eps = values[0] as f64 / values[1] as f64;
    let membrane = abc_iutt::WindowMembrane::new(eps, &values[2..]);
    for &cutoff in &values[2..] {
        match membrane.at(cutoff) {
            Some((value, t)) => println!("cutoff={cutoff} a={} b={} c={} discrepancy={value:.12}", t.a, t.b, t.c),
            None => println!("cutoff={cutoff} empty"),
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
