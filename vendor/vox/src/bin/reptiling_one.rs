#[path = "../reptiling_membrane.rs"] mod reptiling_membrane;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn main() {
    let run = || -> Result<(), String> {
        let values = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
        let inputs: Vec<usize> = values.iter().map(|&n| usize::try_from(n))
            .collect::<Result<_, _>>().map_err(|_| "input exceeds usize")?;
        let membrane = reptiling_membrane::RepTilingMembrane::new(*inputs.iter().max().unwrap())?;
        for n in inputs { println!("n={n} rep_tiling_admissible={}", membrane.at(n).unwrap()); }
        Ok(())
    };
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
