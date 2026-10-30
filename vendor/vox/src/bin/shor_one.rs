//! Baked Shor statevector membrane: membrane_one.sh shor a N qubits.
extern crate alloc;
#[path = "../shor_qft.rs"] mod shor_qft;
#[path = "../membrane_complex.rs"] mod membrane_complex;
#[path = "../baked_membrane.rs"] mod baked_membrane;
fn run() -> Result<(), String> {
    let n = baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))?;
    if n.len() != 3 { return Err("Shor needs a, N, qubits".into()); }
    let bits = usize::try_from(n[2]).map_err(|_| "Qubit count exceeds usize")?;
    let result = shor_qft::simulate_shor(n[0], n[1], bits)?;
    println!("{}", shor_qft::report(&result));
    Ok(())
}
fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
