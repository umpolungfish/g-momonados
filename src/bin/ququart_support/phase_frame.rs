//! Word-only terminal coordinates for one pre-measurement joint-state marginal.
use g_momonados::{
    godel_calculus::{encode_cell_binary, Nat},
    whole_register_phase::WholeRegisterPhaseFrame,
};
use num_bigint::{BigInt, BigUint, Sign};

pub fn record(frame: &WholeRegisterPhaseFrame, digit: &BigUint) -> serde_json::Value {
    let word = |value: &BigUint| {
        encode_cell_binary(&Nat::from_bits_le(
            (0..value.bits()).map(|bit| value.bit(bit)).collect(),
        ))
    };
    let signed = |value: &BigInt| {
        format!(
            "{}{}",
            if value.sign() == Sign::Minus {
                '≺'
            } else {
                '≻'
            },
            word(value.magnitude())
        )
    };
    let coordinates = |values: &[(BigInt, BigInt); 16]| {
        values
            .iter()
            .map(|(re, im)| serde_json::json!({"re_word": signed(re), "im_word": signed(im)}))
            .collect::<Vec<_>>()
    };
    let recovered = &frame.reconstruction;
    serde_json::json!({
        "gram": coordinates(&frame.gram),
        "mass_numerator_words": frame.mass_numerators.iter().map(&word).collect::<Vec<_>>(),
        "recovered_gram": coordinates(&recovered.rounded.recovered),
        "reconstruction_residual_word": word(recovered.rounded.maximum_residual.magnitude()),
        "reconstruction_tolerance_word": word(recovered.rounded.tolerance.magnitude()),
        "reconstruction_denominator_word": word(recovered.denominator.magnitude()),
        "signed_residual_numerators": coordinates(&recovered.signed_residual_numerators),
        "digit_word": word(digit),
    })
}
