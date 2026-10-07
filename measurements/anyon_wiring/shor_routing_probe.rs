//! Wire and operand perturbations bound to >=200-bit RSA-style sources.
extern crate alloc;
pub use g_momonados::fibonacci_shor_plan;
#[path = "../../src/fibonacci_shor.rs"]
mod fibonacci_shor;
use num_bigint::BigUint;
use fibonacci_shor_plan::shor_gate_plan_for_source;

fn main() {
    let input = std::env::args().nth(1).unwrap_or_else(||
        "1156514714917773145849996001252587703581994899993461612691909".into());
    let n = BigUint::parse_bytes(input.as_bytes(), 10).unwrap();
    assert!(n.bits() >= 200);
    let target = 2 * n.bits() as usize;
    let near = fibonacci_shor::controlled_phase_braid(0, 1);
    let far = fibonacci_shor::controlled_phase_braid(0, target);
    let moved = fibonacci_shor::controlled_phase_braid(1, target);
    println!("{{\"source_bits\":{},\"control\":0,\"target\":1,\"word\":{near:?}}}", n.bits());
    println!("{{\"source_bits\":{},\"control\":0,\"target\":{target},\"word\":{far:?}}}", n.bits());
    println!("{{\"source_bits\":{},\"control\":1,\"target\":{target},\"word\":{moved:?}}}", n.bits());
    assert_ne!(near, far);
    println!("{{\"target_change_preserved\":{},\"control_change_preserved\":{}}}", near != far, near != moved);
    let a = shor_gate_plan_for_source(target, &BigUint::from(2u8), &n).unwrap();
    let b = shor_gate_plan_for_source(target, &BigUint::from(8u8), &n).unwrap();
    assert_ne!(a, b);
    println!("{{\"base_change_preserved_in_gate_plan\":true,\"calibrated_braid_available\":false}}");
}
