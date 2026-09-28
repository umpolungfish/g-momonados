//! Baked distinct-triple-sum membrane: membrane_one.sh tripsum limit...
#[path = "../tripsum_membrane.rs"] mod tripsum_membrane;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn run() -> Result<(), String> {
    for n in baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))? {
        let limit = usize::try_from(n).map_err(|_| "Triple-sum limit exceeds usize")?;
        let witness = tripsum_membrane::TripleSumMembrane::solve(limit)?;
        println!("limit={n} size={} witness={witness:?}", witness.len());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
