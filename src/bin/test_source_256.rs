
use g_momonados::arbitrary_factor::*;
fn main() {
    let source_word = std::fs::read_to_string("membranes/ququart_closed_256_radix4_20261005/source.imasm").unwrap().trim().to_string();
    println!("Testing source_word on 256 bits...");
    match native_factor_word_pair(&source_word) {
        Ok(Some((p, q))) => println!("Closed: p={} q={}", p, q),
        Ok(None) => println!("Returned None"),
        Err(e) => println!("Error: {}", e),
    }
}
