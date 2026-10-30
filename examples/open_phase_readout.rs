//! Compiled control for an open terminal on the whole-register phase circuit.
//! The prepared source/base are 15/2; the measurement rank is 5/8. No factor
//! closure runs, and no phase outcome is supplied to circuit preparation.
use g_momonados::{
    ququart_folded_work::PreparedModularWork, reversible_modular::ModularMultiply,
    whole_register_phase::WholeRegisterPhaseCircuit,
};
use num_bigint::BigUint;
use num_traits::ToPrimitive;

#[no_mangle]
#[inline(never)]
pub extern "C" fn open_phase_readout() -> u64 {
    let source = BigUint::from(15u8);
    let base = BigUint::from(2u8);
    let arithmetic = ModularMultiply::new_phase_register(&source).unwrap();
    let mut operations = Vec::new();
    let mut stages = Vec::new();
    let mut multiplier = base.clone();
    for _ in 0..2 {
        let start = operations.len();
        for (lane, power) in [
            (0, multiplier.clone()),
            (1, &multiplier * &multiplier % &source),
        ] {
            arithmetic
                .emit_ququart_nested_radix(&power, lane, 1, |operation| {
                    operations.push(operation);
                    Ok(())
                })
                .unwrap();
        }
        stages.push((multiplier.clone(), (start..operations.len()).collect()));
        let square = &multiplier * &multiplier % &source;
        multiplier = &square * &square % &source;
    }
    let work = PreparedModularWork {
        source: source.clone(),
        digit_bits: 1,
        operations,
        stages,
    };
    let mut circuit =
        WholeRegisterPhaseCircuit::prepare_with_reentry_and_work(source, base, 4, &[], Some(&work))
            .unwrap();
    circuit
        .fire(&BigUint::from(5u8), &BigUint::from(8u8))
        .unwrap()
        .numerator
        .to_u64()
        .unwrap()
}

fn main() {
    println!("whole_register_phase_word={}", open_phase_readout());
}
