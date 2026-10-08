use membranes::{ecm, from_word, mul, to_word};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 4 {
        eprintln!("usage: membrane_ecm_extract <canonical-IMASM-N> [B1] [B2] [curves]");
        std::process::exit(2);
    }
    let Some(n) = from_word(&args[0]) else {
        eprintln!("membrane_ecm_extract: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    let b1 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000u32);
    let b2 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10000u32);
    let curves = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(24u32);
    println!("ECM input bits={}", n.bit_len());
    match ecm::factor(&n, b1, b2.max(b1), curves) {
        Some((p, q)) => println!("factor={}\ncofactor={}\nproduct_closes={}",
            to_word(&p), to_word(&q), mul(&p, &q) == n),
        None => println!("no factor for B1={b1}, B2={}, curves={curves}", b2.max(b1)),
    }
}
