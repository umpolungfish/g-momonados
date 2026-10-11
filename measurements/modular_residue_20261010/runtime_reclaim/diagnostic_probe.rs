#[path = "../../../src/bin/ququart_support/mod.rs"]
mod support;

fn main() {
    let prepared = serde_json::from_str(include_str!("../control128/prepared.json")).unwrap();
    let (_, _, metrics) = support::contract(&prepared).unwrap();
    println!("computational_bits={:016x}", metrics.computational.to_bits());
    println!("leakage_bits={:016x}", metrics.leakage.to_bits());
    println!("closure_bits={:016x}", metrics.closure.to_bits());
    println!("exchanges={}", metrics.exchanges);
}
