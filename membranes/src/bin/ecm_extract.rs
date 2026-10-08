use membranes::{divmod, ecm, from_word, is_zero, to_word};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 5 {
        eprintln!("usage: membrane_ecm_extract <canonical-IMASM-N> [B1] [B2] [curves] [first-sigma]");
        std::process::exit(2);
    }
    let Some(n) = from_word(&args[0]) else {
        eprintln!("membrane_ecm_extract: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    let b1 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000u32);
    let b2 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000u32);
    let curves = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(24u32);
    let first_sigma = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6u32);
    println!("ECM input bits={}", n.bit_len());
    match ecm::factor_from_sigma(&n, b1, b2.max(b1), curves, first_sigma) {
        Some(factor) => {
            let (_, remainder) = divmod(&n, &factor);
            println!("factor={}\nproduct_closes={}", to_word(&factor), is_zero(&remainder));
        }
        None => println!("no factor for B1={b1}, B2={}, curves={curves}", b2.max(b1)),
    }
}
