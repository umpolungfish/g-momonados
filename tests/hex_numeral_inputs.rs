use g_momonados::{factor_membrane, godel_analyzer, godel_calculus, native_numeral,
    ququart_factor, word_tape::WordTape};
use num_bigint::BigUint;
use vox_core::combined_factor as codec;

#[test]
fn native_hex_inputs_share_the_executing_numeric_boundaries() {
    let wide: BigUint = "1218745108538121657062000951884568623049596243192565916939841".parse().unwrap();
    for n in (0u32..16).map(BigUint::from).chain([wide.clone(), (&wide << 700usize)+BigUint::from(1u8)]) {
        let native = codec::native_word(&n);
        let hex = codec::hex_word(&n);
        assert_eq!(g_momonados::numeral_encodings::digit_encode(&n.to_string()), hex);
        assert_eq!(native_numeral::encode(&n.to_string()), native);
        assert_eq!(WordTape::from_canonical_word(&native), WordTape::from_canonical_word(&hex));
        assert_eq!(native_numeral::decode(&hex), Some(n.clone()));
        assert_eq!(native_numeral::canonicalize_input(&hex).unwrap(), native);
        assert_eq!(godel_calculus::decode_cell_biguint(&hex).unwrap(), n);
        assert_eq!(godel_analyzer::parse_input(&hex).unwrap().to_string(), n.to_string());
        assert_eq!(factor_membrane::parse_word_cells(&native), factor_membrane::parse_word_cells(&hex));
        assert_eq!(native_numeral::interlace_words(&native, &native).unwrap(), native_numeral::interlace_words(&hex, &hex).unwrap());
        assert_eq!(native_numeral::deinterlace_word(&native).unwrap(), native_numeral::deinterlace_word(&hex).unwrap());
        assert_eq!(factor_membrane::repl_factor_membrane(&["encode", &native]), factor_membrane::repl_factor_membrane(&["encode", &hex]));
        assert!(WordTape::from_canonical_word(&(codec::HEX_WORDS[0].to_owned()+&hex)).is_none());
    }
    let radix = BigUint::from(256u32);
    assert_eq!(ququart_factor::power_of_two_radix_word(&codec::native_word(&radix)).unwrap(),
        ququart_factor::power_of_two_radix_word(&codec::hex_word(&radix)).unwrap());
}
