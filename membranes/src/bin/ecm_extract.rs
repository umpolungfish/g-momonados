use membranes::{divmod, ecm, from_word, is_zero, to_word};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input = None;
    let mut b1 = 1000u32;
    let mut b2 = 10000u32;
    let mut curves = 24u32;
    let mut first_sigma = 6u32;
    let mut index = 0;
    while index < args.len() {
        let value = args.get(index + 1).unwrap_or_else(|| {
            eprintln!("each flag requires a value");
            std::process::exit(2);
        });
        match args[index].as_str() {
            "--number" => input = Some(value.as_str()),
            "--b1" => b1 = value.parse().unwrap_or_else(|_| { eprintln!("--b1 must be an integer"); std::process::exit(2); }),
            "--b2" => b2 = value.parse().unwrap_or_else(|_| { eprintln!("--b2 must be an integer"); std::process::exit(2); }),
            "--curves" => curves = value.parse().unwrap_or_else(|_| { eprintln!("--curves must be an integer"); std::process::exit(2); }),
            "--first-sigma" => first_sigma = value.parse().unwrap_or_else(|_| { eprintln!("--first-sigma must be an integer"); std::process::exit(2); }),
            flag => { eprintln!("unknown flag {flag}"); std::process::exit(2); }
        }
        index += 2;
    }
    let Some(input) = input else {
        eprintln!("usage: membrane_ecm_extract --number CANONICAL_IMASM_N [--b1 B1] [--b2 B2] [--curves COUNT] [--first-sigma SIGMA]");
        std::process::exit(2);
    };
    let Some(n) = from_word(input) else {
        eprintln!("membrane_ecm_extract: expected a canonical IMASM numeral word");
        std::process::exit(2);
    };
    println!("ECM input bits={}", n.bit_len());
    match ecm::factor_from_sigma(&n, b1, b2.max(b1), curves, first_sigma) {
        Some(factor) => {
            let (_, remainder) = divmod(&n, &factor);
            println!("factor={}\nproduct_closes={}", to_word(&factor), is_zero(&remainder));
        }
        None => println!("no factor for B1={b1}, B2={}, curves={curves}", b2.max(b1)),
    }
}
