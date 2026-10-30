//! Baked divisor-ring membrane. Usage: membrane_one.sh divisor N...
extern crate alloc;
#[path = "../divisor_ring.rs"] mod divisor_ring;
#[path = "../baked_membrane.rs"] mod baked_membrane;

fn run() -> Result<(), String> {
    for n in baked_membrane::numbers(option_env!("MEMBRANE_WORDS"))? {
        let result = divisor_ring::analyze(n);
        println!("{}", divisor_ring::format_report(&result));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() { eprintln!("{error}"); std::process::exit(2); }
}
