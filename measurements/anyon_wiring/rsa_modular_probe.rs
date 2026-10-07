//! Check emitted reversible gates against independent large-integer arithmetic.
use g_momonados::reversible_modular::{ElementaryGate, ModularMultiply};
use num_bigint::BigUint;
use num_traits::{One, Zero};

fn main() {
    let decimal = std::env::args().nth(1).expect("source modulus required");
    let n = BigUint::parse_bytes(decimal.as_bytes(), 10).expect("decimal source");
    assert!(n.bits() >= 200, "only RSA-style sources of at least 200 bits");
    let circuit = ModularMultiply::new(&n).unwrap();
    let values = [BigUint::zero(), BigUint::one(), &n - BigUint::one(), &n / 3u8];
    for multiplier in [BigUint::from(2u8), BigUint::from(8u8), &n - BigUint::from(2u8)] {
        let mut states = Vec::new();
        for control in [false, true] {
            for value in &values {
                let mut bits = vec![false; circuit.elementary_qubits()];
                bits[0] = control;
                for bit in 0..circuit.width() { bits[bit + 1] = value.bit(bit as u64); }
                states.push((control, value.clone(), bits));
            }
        }
        let mut count = 0u64;
        circuit.emit_elementary(&multiplier, |gate| {
            for (_, _, bits) in &mut states {
                match gate {
                    ElementaryGate::X(target) => bits[target] = !bits[target],
                    ElementaryGate::Cnot { control, target } => bits[target] ^= bits[control],
                    ElementaryGate::Toffoli { first, second, target } => bits[target] ^= bits[first] && bits[second],
                }
            }
            count += 1;
            Ok(())
        }).unwrap();
        for (control, input, bits) in states {
            let mut output = BigUint::zero();
            for bit in 0..circuit.width() {
                if bits[bit + 1] { output |= BigUint::one() << bit; }
            }
            let expected = if control { &input * &multiplier % &n } else { input.clone() };
            assert_eq!(output, expected, "emitted modular permutation");
            assert_eq!(bits[0], control, "control wire preserved");
            assert!(bits[circuit.width() + 1..].iter().all(|bit| !*bit), "all arithmetic workspace returned clean");
        }
        println!("source_bits={} multiplier={} gates={} checked_inputs={} scratch_clean=true", n.bits(), multiplier, count, values.len() * 2);
    }
}
