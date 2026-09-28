#[path = "../prime_power_membrane.rs"] mod prime_power_membrane;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn run() -> Result<(), String> {
    let values = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
    let sizes: Vec<usize> = values.iter().map(|&n| usize::try_from(n))
        .collect::<Result<_, _>>().map_err(|_| "Input exceeds usize")?;
    let table = prime_power_membrane::PrimePowerMembrane::new(*sizes.iter().max().unwrap())?;
    for n in sizes {
        match table.lcm(n).unwrap() {
            Some(value) => println!("n={n} lcm_1_to_n={value}"),
            None => println!("n={n} lcm_1_to_n=overflow_u128"),
        }
    }
    Ok(())
}
fn main() { if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); } }
