//! Baked Landau membrane: membrane_one.sh landau N...
#[path = "../landau_membrane.rs"] mod landau_membrane;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn run() -> Result<(), String> {
    let values = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
    let sizes: Result<Vec<_>, _> = values.iter().map(|&n| usize::try_from(n)).collect();
    let sizes = sizes.map_err(|_| "Landau input exceeds usize")?;
    let membrane = landau_membrane::LandauMembrane::new(*sizes.iter().max().unwrap())?;
    for n in sizes { println!("n={n} largest_partition_lcm={}", membrane.at(n).unwrap()); }
    Ok(())
}
fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
