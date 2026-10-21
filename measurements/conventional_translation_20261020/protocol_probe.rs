use g_momonados::conventional_spectrum;
fn main() {
    let source=std::env::args().nth(1).expect("N-only source file");
    let text=std::fs::read_to_string(source).unwrap();
    let receipt=conventional_spectrum::source_protocol_control(text.trim().parse().unwrap())
        .unwrap_or_else(|error|panic!("protocol execution control: {error}"));
    println!("{receipt}");
}
