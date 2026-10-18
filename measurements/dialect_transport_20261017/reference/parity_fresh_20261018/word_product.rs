
use g_momonados::word_tape::WordTape;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let left = WordTape::from_canonical_word(std::fs::read_to_string(&args[1]).unwrap().trim()).unwrap();
    let right = WordTape::from_canonical_word(std::fs::read_to_string(&args[2]).unwrap().trim()).unwrap();
    let source = left.mul(&right);
    let (quotient, remainder) = source.divmod(&left).unwrap();
    assert!(remainder.is_zero());
    assert_eq!(quotient, right);
    println!("{{\"source_word\":\"{}\",\"source_bits\":{},\"word_product_and_quotient\":true}}",
        source.as_word(), source.bit_len());
}
