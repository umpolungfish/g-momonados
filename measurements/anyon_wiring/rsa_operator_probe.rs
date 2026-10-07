//! Contract an actual emitted braid, recover its coordinates, and audit entry.
#[path = "../../src/bin/ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;
use g_momonados::{anyon_pair::{FibonacciPair, PairMatrix, LEAKAGE_CHANNEL},
    godel_calculus::{encode_cell_binary, Nat}};
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, ToPrimitive};
use serde_json::{json, Value};

fn word(n: &BigUint) -> String {
    encode_cell_binary(&Nat::from_bits_le((0..n.bits()).map(|b| n.bit(b)).collect()))
}
fn signed(n: &BigInt) -> String {
    format!("{}{}", if n.is_negative() { "≺" } else { "≻" }, word(n.magnitude()))
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n = BigUint::parse_bytes(args[1].as_bytes(), 10).unwrap();
    assert!(n.bits() >= 200);
    let exchanges: Vec<i32> = std::fs::read_to_string(&args[2]).unwrap()
        .split_whitespace().map(|g| g.parse().unwrap()).collect();
    assert!(!exchanges.is_empty());
    let pair = FibonacciPair::new(&n).unwrap();
    let physical = pair.evaluate(&exchanges).unwrap();
    let observed = pair.in_pair_channels(&physical);
    let inverse: Vec<i32> = exchanges.iter().rev().map(|g| -*g).collect();
    let returned = pair.evaluate(&inverse).unwrap().multiply(&physical, pair.format());
    let identity = PairMatrix::identity(pair.format());
    let scale = pair.format().scale();
    let physical_closure = returned.0.iter().zip(&identity.0).map(|(a, b)| {
        (&a.re - &b.re).abs().max((&a.im - &b.im).abs()).to_f64().unwrap()
            / scale.to_f64().unwrap()
    }).fold(0.0f64, f64::max);
    assert!(physical_closure < 2.0f64.powi(-4));
    let entries: Vec<Value> = observed.0.iter().map(|z|
        json!({"re_word": signed(&z.re), "im_word": signed(&z.im)})).collect();
    let scalar = |v: u64| word(&BigUint::from(v));
    let mut prepared = json!({"source_word": word(&n), "accuracy_word": scalar(4),
        "prepared_operator": {"w_bits_word": scalar(pair.format().w_bits), "matrix": entries,
            "computational_word": scalar(0), "leakage_word": scalar(0),
            "closure_word": scalar(physical_closure.to_bits()),
            "exchanges_word": scalar(exchanges.len() as u64)}});
    // Zero saved action diagnostics must not replace the actual measurements.
    let (decoded_n, decoded, metrics) = support::contract(&prepared).unwrap();
    assert_eq!(decoded_n, n);
    assert_eq!(decoded.0, observed.0);
    assert!(metrics.computational > 0.0);
    assert_eq!(metrics.exchanges, exchanges.len());
    prepared["prepared_operator"]["computational_word"] = json!(scalar(metrics.computational.to_bits()));
    prepared["prepared_operator"]["leakage_word"] = json!(scalar(metrics.leakage.to_bits()));
    prepared["prepared_operator"]["closure_word"] = json!(scalar(metrics.closure.to_bits()));
    std::fs::write(&args[3], serde_json::to_vec(&prepared).unwrap()).unwrap();
    // Each mutation retains canonical coordinates and falsely claims zero
    // diagnostics. Entry must still reject the changed executable map.
    for (label, index) in [("computational", 0), ("leakage", 5 * LEAKAGE_CHANNEL),
                            ("return", 5 * LEAKAGE_CHANNEL + LEAKAGE_CHANNEL)] {
        let mut changed = prepared.clone();
        changed["prepared_operator"]["matrix"][index]["re_word"] =
            json!(signed(&(&observed.0[index].re + &scale)));
        for field in ["computational_word", "leakage_word", "closure_word"] {
            changed["prepared_operator"][field] = json!(scalar(0));
        }
        assert!(support::contract(&changed).is_err(), "{label} mutation entered resident state");
    }
    println!("source_bits={} physical_generators={} all_coordinates_recovered=true actual_computational={:.8e} actual_leakage={:.8e} bidirectional_return={:.8e} physical_inverse_return={:.8e} changed_maps_rejected=3",
        n.bits(), exchanges.len(), metrics.computational, metrics.leakage, metrics.closure, physical_closure);
}
