//! Baked Schutte membrane: membrane_one.sh schutte vertices subset_size...
#[path = "../schutte_membrane.rs"] mod schutte_membrane;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn run() -> Result<(), String> {
    let n = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
    if n.len() < 2 { return Err("Schutte needs a vertex count and subset sizes".into()); }
    let vertices = u32::try_from(n[0]).map_err(|_| "Vertex count exceeds u32")?;
    let membrane = schutte_membrane::SchutteMembrane::new(vertices)?;
    for &k in &n[1..] {
        let k = u32::try_from(k).map_err(|_| "Subset size exceeds u32")?;
        let r = membrane.check(k);
        println!("vertices={vertices} k={k} holds={} subsets={} counterexample={:?}",
            r.holds, r.subsets_checked, r.counterexample);
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
